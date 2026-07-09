//! # [`IdentifyNodes`]
//!
//! This pass populates the graph with nodes using the input given in [`CompilerInput`].
//! This pass is *mandatory*. Without it, the graph will never be populated.
//!
//! If `optimize` is set in [`CompilerOptions`], redstone wires will not be added to the graph.
//!
//! There are no requirements for this pass.

use crate::compile_graph::{Annotations, CompileGraph, CompileNode, NodeIdx, NodeState, NodeType};
use crate::passes::{AnalysisInfos, Pass};
use crate::{CompilerInput, CompilerOptions};
use itertools::Itertools;
use mchprs_blocks::block_entities::BlockEntity;
use mchprs_blocks::blocks::Block;
use mchprs_blocks::{BlockDirection, BlockFace, BlockPos};
use mchprs_redstone::{self, comparator, noteblock, wire};
use mchprs_world::{for_each_block_optimized, World};
use rustc_hash::{FxHashMap, FxHashSet};
use serde_json::Value;
use tracing::warn;

pub struct IdentifyNodes;

/// Find all wires that are connected to custom IO wires
/// This is needed when optimization is enabled to ensure custom IO wires can propagate power
fn find_wires_connected_to_custom_io<W: World>(
    world: &W,
    custom_io: &[BlockPos],
    bounds: (BlockPos, BlockPos),
) -> FxHashSet<BlockPos> {
    use std::collections::VecDeque;
    
    let mut connected_wires = FxHashSet::default();
    
    for &custom_io_pos in custom_io {
        // Only process if this custom IO position is actually a wire
        if !matches!(world.get_block(custom_io_pos), Block::RedstoneWire(_)) {
            continue;
        }
        
        // BFS to find all connected wires
        let mut queue = VecDeque::new();
        queue.push_back(custom_io_pos);
        connected_wires.insert(custom_io_pos);
        
        while let Some(pos) = queue.pop_front() {
            // Check all 6 directions for adjacent wires
            for face in &BlockFace::values() {
                let neighbor_pos = pos.offset(*face);
                
                // Skip if out of bounds
                if neighbor_pos.x < bounds.0.x || neighbor_pos.x > bounds.1.x
                    || neighbor_pos.y < bounds.0.y || neighbor_pos.y > bounds.1.y
                    || neighbor_pos.z < bounds.0.z || neighbor_pos.z > bounds.1.z
                {
                    continue;
                }
                
                // Skip if already visited
                if connected_wires.contains(&neighbor_pos) {
                    continue;
                }
                
                // Check if it's a wire
                if matches!(world.get_block(neighbor_pos), Block::RedstoneWire(_)) {
                    connected_wires.insert(neighbor_pos);
                    queue.push_back(neighbor_pos);
                }
            }
        }
    }
    
    connected_wires
}

impl<W: World> Pass<W> for IdentifyNodes {
    fn run_pass(
        &self,
        graph: &mut CompileGraph,
        options: &CompilerOptions,
        input: &CompilerInput<'_, W>,
        _: &mut AnalysisInfos,
    ) {
        let ignore_wires = options.optimize;
        let plot = input.world;

        let mut first_pass = FxHashMap::default();
        let mut second_pass = FxHashSet::default();

        let (first_pos, second_pos) = input.bounds;
        
        // Find all wires connected to custom IO wires
        let wires_connected_to_custom_io = if ignore_wires && !options.custom_io.is_empty() {
            find_wires_connected_to_custom_io(plot, &options.custom_io, (first_pos, second_pos))
        } else {
            FxHashSet::default()
        };

        for_each_block_optimized(plot, first_pos, second_pos, |pos| {
            for_pos(
                graph,
                &mut first_pass,
                &mut second_pass,
                ignore_wires,
                options.wire_dot_out,
                &options.custom_io,
                &wires_connected_to_custom_io,
                plot,
                pos,
            );
        });

        for pos in second_pass {
            apply_annotations(graph, options, &first_pass, plot, pos);
        }
    }

    fn status_message(&self) -> &'static str {
        "Identifying nodes"
    }

    fn driver_key(&self) -> &'static str {
        "identify-nodes"
    }
}

fn for_pos<W: World>(
    graph: &mut CompileGraph,
    first_pass: &mut FxHashMap<BlockPos, NodeIdx>,
    second_pass: &mut FxHashSet<BlockPos>,
    ignore_wires: bool,
    wire_dot_out: bool,
    custom_io: &[BlockPos],
    wires_connected_to_custom_io: &FxHashSet<BlockPos>,
    world: &W,
    pos: BlockPos,
) {
    let id = world.get_block_raw(pos);
    let block = Block::from_id(id);

    if block.is_sign() || block.is_wall_sign() {
        second_pass.insert(pos);
        return;
    }

    let Some((ty, state)) = identify_block(block, pos, world) else {
        return;
    };

    let is_custom_io = custom_io.contains(&pos);

    // Check if this wire is connected to a custom IO wire
    // These wires need to be included in the graph even with optimization enabled
    // so that custom IO wires can propagate power through them
    let is_connected_to_custom_io = ty == NodeType::Wire && wires_connected_to_custom_io.contains(&pos);

    // Custom IO: Keep original node type (Wire/Repeater/Comparator/etc) but mark as input/output
    // This allows ANY component to be monitored or controlled via custom IO
    let is_input = ty.is_normally_input() || is_custom_io || is_connected_to_custom_io;
    let is_output = ty.is_normally_output()
        || matches!(block, Block::RedstoneWire(wire) if wire_dot_out && wire::is_dot(wire))
        || is_custom_io
        || is_connected_to_custom_io;

    if ignore_wires && ty == NodeType::Wire && !(is_input | is_output) {
        return;
    }

    let node_idx = graph.add_node(CompileNode {
        ty,
        block: Some((pos, id)),
        aliased_blocks: Vec::new(),
        name: None,
        state,

        is_input,
        is_output,
        annotations: Annotations::default(),
    });
    first_pass.insert(pos, node_idx);
}

fn identify_block<W: World>(
    block: Block,
    pos: BlockPos,
    world: &W,
) -> Option<(NodeType, NodeState)> {
    if let Some(powered) = block.clone().get_pressure_plate_powered() {
        return Some((NodeType::PressurePlate, NodeState::simple(*powered)));
    }
    let (ty, state) = match block {
        Block::Repeater(repeater) => (
            NodeType::Repeater {
                delay: repeater.delay,
                facing_diode: mchprs_redstone::is_diode(
                    world.get_block(pos.offset(repeater.facing.opposite().block_face())),
                ),
            },
            NodeState::repeater(repeater.powered, repeater.locked),
        ),
        Block::Comparator(comparator) => (
            NodeType::Comparator {
                mode: comparator.mode,
                far_input: comparator::get_far_input(world, pos, comparator.facing),
                facing_diode: mchprs_redstone::is_diode(
                    world.get_block(pos.offset(comparator.facing.opposite().block_face())),
                ),
            },
            NodeState::comparator(
                comparator.powered,
                if let Some(BlockEntity::Comparator { output_strength }) =
                    world.get_block_entity(pos)
                {
                    *output_strength
                } else {
                    0
                },
            ),
        ),
        Block::RedstoneTorch { lit, .. } | Block::RedstoneWallTorch { lit, .. } => {
            (NodeType::Torch, NodeState::simple(lit))
        }
        Block::RedstoneWire(wire) => (NodeType::Wire, NodeState::ss(wire.power)),
        Block::StoneButton { powered, .. } => (NodeType::Button, NodeState::simple(powered)),
        Block::RedstoneLamp { lit } => (NodeType::Lamp, NodeState::simple(lit)),
        Block::Lever { powered, .. } => (NodeType::Lever, NodeState::simple(powered)),
        Block::IronTrapdoor { powered, .. } => (NodeType::Trapdoor, NodeState::simple(powered)),
        Block::RedstoneBlock => (NodeType::Constant, NodeState::ss(15)),
        Block::NoteBlock {
            instrument: _,
            note,
            powered,
        } if noteblock::is_noteblock_unblocked(world, pos) => {
            let instrument = noteblock::get_noteblock_instrument(world, pos);
            (
                NodeType::NoteBlock { instrument, note },
                NodeState::simple(powered),
            )
        }
        block if comparator::has_override(block) => (
            NodeType::Constant,
            NodeState::ss(comparator::get_override(block, world, pos)),
        ),
        Block::Observer { facing, powered } => {
            (NodeType::Observer { facing }, NodeState::simple(powered))
        }
        Block::PoweredRail(rail) => (NodeType::PoweredRail, NodeState::simple(rail.powered)),
        Block::ActivatorRail(rail) => (NodeType::ActivatorRail, NodeState::simple(rail.powered)),
        _ => return None,
    };
    Some((ty, state))
}

fn apply_annotations<W: World>(
    graph: &mut CompileGraph,
    options: &CompilerOptions,
    first_pass: &FxHashMap<BlockPos, NodeIdx>,
    world: &W,
    pos: BlockPos,
) {
    let block = world.get_block(pos);
    let annotations = parse_sign_annotations(world.get_block_entity(pos));
    if annotations.is_empty() {
        return;
    }

    let targets = match (block.get_sign_rotation(), block.get_wall_sign_facing()) {
        (Some(rotation), None) => {
            if let Some(facing) = BlockDirection::from_rotation(rotation) {
                let behind = pos.offset(facing.opposite().block_face());
                vec![behind]
            } else {
                warn!("Found sign with annotations, but bad rotation at {}", pos);
                return;
            }
        }
        (None, Some(facing)) => {
            let behind = pos.offset(facing.opposite().block_face());
            vec![
                behind,
                behind.offset(BlockFace::Top),
                behind.offset(BlockFace::Bottom),
            ]
        }
        _ => panic!("Block unimplemented for second pass"),
    };

    let target = targets.iter().flat_map(|pos| first_pass.get(pos)).next();
    if let Some(&node_idx) = target {
        for annotation in annotations {
            let result = annotation.apply(graph, node_idx, options);
            if let Err(msg) = result {
                warn!("{} at {}", msg, pos);
            }
        }
    } else {
        warn!("Could not find component for annotation at {}", pos);
    }
}

fn parse_sign_annotations(entity: Option<&BlockEntity>) -> Vec<NodeAnnotation> {
    if let Some(BlockEntity::Sign(sign)) = entity {
        sign.front_rows
            .iter()
            .flat_map(|row| serde_json::from_str(row))
            .flat_map(|json: Value| NodeAnnotation::parse(json.as_object()?.get("text")?.as_str()?))
            .collect_vec()
    } else {
        vec![]
    }
}

pub enum NodeAnnotation {}

impl NodeAnnotation {
    fn parse(s: &str) -> Option<Self> {
        let s = s.trim().to_ascii_lowercase();
        if !(s.starts_with('[') && s.ends_with(']')) {
            return None;
        }
        let _parts = s[1..s.len() - 1].split(' ').collect_vec();
        None
    }

    fn apply(
        self,
        _graph: &mut CompileGraph,
        _node_idx: NodeIdx,
        _options: &CompilerOptions,
    ) -> Result<(), String> {
        match self {}
    }
}
