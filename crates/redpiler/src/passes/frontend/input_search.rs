//! # [`InputSearch`]
//!
//! This pass populates the graph with edges.
//! This pass is *mandatory*. Without it, there would be no links between nodes.

use crate::compile_graph::{CompileGraph, CompileLink, LinkType, NodeIdx};
use crate::passes::{AnalysisInfos, Pass};
use crate::{CompilerInput, CompilerOptions};
use mchprs_blocks::blocks::{Block, LeverFace, StraightRailShape};
use mchprs_blocks::{BlockDirection, BlockFace, BlockPos};
use mchprs_redstone::{self, comparator, wire};
use mchprs_world::World;
use rustc_hash::FxHashMap;
use std::collections::VecDeque;

pub struct InputSearch;

impl<W: World> Pass<W> for InputSearch {
    fn run_pass(
        &self,
        graph: &mut CompileGraph,
        options: &CompilerOptions,
        input: &CompilerInput<'_, W>,
        _: &mut AnalysisInfos,
    ) {
        let mut state = InputSearchState::new(input.world, graph, &options.custom_io);
        state.search();
    }

    fn status_message(&self) -> &'static str {
        "Searching for links"
    }

    fn driver_key(&self) -> &'static str {
        "input-search"
    }
}

/// Querying blocks from the world may be expensive, so we save recently
/// queried blocks in a cache to reduce the number of times we query the world.
struct BlockLookupCache<'world, W: World> {
    world: &'world W,
    // It's not really that complex
    #[allow(clippy::type_complexity)]
    cache: [[[Option<(Block, BlockPos)>; 16]; 16]; 16],
}

impl<'world, W: World> BlockLookupCache<'world, W> {
    pub fn new(world: &'world W) -> Self {
        Self {
            world,
            cache: Default::default(),
        }
    }

    fn get_block(&mut self, pos: BlockPos) -> Block {
        let cache_x = pos.x as usize % 16;
        let cache_y = pos.y as usize % 16;
        let cache_z = pos.z as usize % 16;
        let cache_entry = &mut self.cache[cache_x][cache_y][cache_z];
        match cache_entry {
            Some((block, block_pos)) if *block_pos == pos => *block,
            _ => {
                let block = self.world.get_block(pos);
                *cache_entry = Some((block, pos));
                block
            }
        }
    }
}

struct InputSearchState<'a, W: World> {
    world: &'a W,
    graph: &'a mut CompileGraph,
    pos_map: FxHashMap<BlockPos, NodeIdx>,
    custom_io: &'a [BlockPos],
    block_lookup_cache: BlockLookupCache<'a, W>,
}

impl<'a, W: World> InputSearchState<'a, W> {
    fn new(world: &'a W, graph: &'a mut CompileGraph, custom_io: &'a [BlockPos]) -> InputSearchState<'a, W> {
        let mut pos_map = FxHashMap::default();
        for id in graph.node_indices() {
            if let Some((pos, _)) = graph[id].block {
                pos_map.insert(pos, id);
            }
            for (pos, _) in &graph[id].aliased_blocks {
                pos_map.insert(*pos, id);
            }
        }

        InputSearchState {
            world,
            graph,
            pos_map,
            custom_io,
            block_lookup_cache: BlockLookupCache::new(world),
        }
    }

    // unfortunate
    #[allow(clippy::too_many_arguments)]
    fn get_redstone_links(
        &mut self,
        block: Block,
        side: BlockFace,
        pos: BlockPos,
        link_ty: LinkType,
        distance: u8,
        start_node: NodeIdx,
        search_wire: bool,
    ) {
        if block.is_solid() {
            for side in &BlockFace::values() {
                let pos = pos.offset(*side);
                let block = self.block_lookup_cache.get_block(pos);
                if provides_strong_power(block, *side) {
                    self.graph.add_edge(
                        self.pos_map[&pos],
                        start_node,
                        CompileLink::new(link_ty, distance),
                    );
                }

                if let Block::RedstoneWire(wire) = block {
                    if !search_wire {
                        continue;
                    }
                    match side {
                        BlockFace::Top => {
                            self.search_wire(start_node, pos, link_ty, distance);
                        }
                        BlockFace::Bottom => {}
                        _ => {
                            let direction = side.unwrap_direction();
                            if search_wire
                                && !wire::get_current_side(
                                    wire::get_regulated_sides(wire, self.world, pos),
                                    direction.opposite(),
                                )
                                .is_none()
                            {
                                self.search_wire(start_node, pos, link_ty, distance);
                            }
                        }
                    }
                }
            }
        } else if provides_weak_power(block, side) {
            if let Some(&from_node) = self.pos_map.get(&pos) {
                // Don't create self-loops
                if from_node != start_node {
                    self.graph.add_edge(
                        from_node,
                        start_node,
                        CompileLink::new(link_ty, distance),
                    );
                }
            }
        } else if let Block::RedstoneWire(wire) = block {
            match side {
                BlockFace::Top => self.search_wire(start_node, pos, link_ty, distance),
                BlockFace::Bottom => {}
                _ => {
                    let direction = side.unwrap_direction();
                    if search_wire
                        && !wire::get_current_side(
                            wire::get_regulated_sides(wire, self.world, pos),
                            direction.opposite(),
                        )
                        .is_none()
                    {
                        self.search_wire(start_node, pos, link_ty, distance);
                    }
                }
            }
        }
    }

    fn search_wire(
        &mut self,
        start_node: NodeIdx,
        root_pos: BlockPos,
        link_ty: LinkType,
        initial_distance: u8,
    ) {
        let mut discovered = vec![(root_pos, initial_distance)];

        // Linear search has better runtime performance than any other lookup when there are
        // only a few elements, which is most of the time.
        let has_been_discovered =
            |discovered: &[_], new_pos| discovered.iter().any(|(pos, _)| *pos == new_pos);

        let mut idx = 0;
        while idx < discovered.len() {
            let (pos, distance) = discovered[idx];
            idx += 1;

            // We can stop looking once we've reached the max ss of a wire.
            // This also prevents overflowing the distance past 255.
            if distance > 15 {
                continue;
            }

            // The block above the wire. If it's solid, we can't connect up diagonally
            let up_pos = pos.offset(BlockFace::Top);
            let up_block = self.block_lookup_cache.get_block(up_pos);

            for side in &BlockFace::values() {
                let neighbor_pos = pos.offset(*side);
                let neighbor = self.block_lookup_cache.get_block(neighbor_pos);

                self.get_redstone_links(
                    neighbor,
                    *side,
                    neighbor_pos,
                    link_ty,
                    distance,
                    start_node,
                    false,
                );

                if is_wire(neighbor) && !has_been_discovered(&discovered, neighbor_pos) {
                    discovered.push((neighbor_pos, distance + 1));
                }

                if side.is_horizontal() {
                    if !up_block.is_solid() && !neighbor.is_transparent() {
                        let neighbor_up_pos = neighbor_pos.offset(BlockFace::Top);
                        if is_wire(self.block_lookup_cache.get_block(neighbor_up_pos))
                            && !has_been_discovered(&discovered, neighbor_up_pos)
                        {
                            discovered.push((neighbor_up_pos, distance + 1));
                        }
                    }

                    if !neighbor.is_solid() {
                        let neighbor_down_pos = neighbor_pos.offset(BlockFace::Bottom);
                        if is_wire(self.block_lookup_cache.get_block(neighbor_down_pos))
                            && !has_been_discovered(&discovered, neighbor_down_pos)
                        {
                            discovered.push((neighbor_down_pos, distance + 1));
                        }
                    }
                }
            }
        }
    }

    /// Create output edges from a custom IO wire to all connected wires and components
    /// This allows custom IO wires to act as power sources when set via set_signal_strength()
    fn create_wire_output_edges(&mut self, wire_node: NodeIdx, wire_pos: BlockPos) {
        // Do a BFS to find all connected wires and components
        let mut queue: VecDeque<BlockPos> = VecDeque::new();
        let mut discovered = FxHashMap::default();

        discovered.insert(wire_pos, 0u8);
        queue.push_back(wire_pos);

        while let Some(pos) = queue.pop_front() {
            let distance = discovered[&pos];

            // Stop at max wire distance
            if distance > 15 {
                continue;
            }

            let up_pos = pos.offset(BlockFace::Top);
            let up_block = self.world.get_block(up_pos);

            for side in &BlockFace::values() {
                let neighbor_pos = pos.offset(*side);
                let neighbor = self.world.get_block(neighbor_pos);

                // Add edge to neighboring wire if it exists in the graph
                if is_wire(neighbor) {
                    if let Some(&neighbor_node) = self.pos_map.get(&neighbor_pos) {
                        if !discovered.contains_key(&neighbor_pos) {
                            self.graph.add_edge(
                                wire_node,
                                neighbor_node,
                                CompileLink::new(LinkType::Default, distance + 1),
                            );
                            queue.push_back(neighbor_pos);
                            discovered.insert(neighbor_pos, distance + 1);
                        }
                    }
                }

                // Add edges to components that can be powered by wires
                if let Some(&neighbor_node) = self.pos_map.get(&neighbor_pos) {
                    // Components that receive power from adjacent wires
                    match neighbor {
                        Block::RedstoneLamp { .. } | Block::IronTrapdoor { .. } | Block::NoteBlock { .. } => {
                            self.graph.add_edge(
                                wire_node,
                                neighbor_node,
                                CompileLink::new(LinkType::Default, distance),
                            );
                        }
                        Block::RedstoneTorch { .. } if *side == BlockFace::Top => {
                            // Torch on top of block above wire
                            self.graph.add_edge(
                                wire_node,
                                neighbor_node,
                                CompileLink::new(LinkType::Default, distance),
                            );
                        }
                        Block::Repeater(repeater) if repeater.facing.opposite().block_face() == *side => {
                            // Repeater facing the wire
                            self.graph.add_edge(
                                wire_node,
                                neighbor_node,
                                CompileLink::new(LinkType::Default, distance),
                            );
                        }
                        Block::Comparator(comparator) if comparator.facing.opposite().block_face() == *side => {
                            // Comparator facing the wire (rear/back input)
                            self.graph.add_edge(
                                wire_node,
                                neighbor_node,
                                CompileLink::new(LinkType::Default, distance),
                            );
                        }
                        Block::Comparator(comparator) => {
                            // Check if wire is on the comparator's side (left or right)
                            let comp_left = comparator.facing.rotate_ccw().block_face();
                            let comp_right = comparator.facing.rotate().block_face();
                            
                            if *side == comp_left || *side == comp_right {
                                // Wire is on comparator's side - create SIDE edge
                                self.graph.add_edge(
                                    wire_node,
                                    neighbor_node,
                                    CompileLink::new(LinkType::Side, distance),
                                );
                            }
                        }
                        _ => {}
                    }
                }

                // Wires also power solid blocks, which can then power components on top/around them
                if neighbor.is_solid() {
                    // Check all faces of the solid block for components that can be powered
                    for block_side in &BlockFace::values() {
                        let component_pos = neighbor_pos.offset(*block_side);
                        if let Some(&component_node) = self.pos_map.get(&component_pos) {
                            let component_block = self.world.get_block(component_pos);
                            match component_block {
                                Block::RedstoneTorch { .. } if *block_side == BlockFace::Top => {
                                    // Torch on top of the solid block
                                    self.graph.add_edge(
                                        wire_node,
                                        component_node,
                                        CompileLink::new(LinkType::Default, distance),
                                    );
                                }
                                Block::RedstoneWallTorch { facing, .. } if facing.block_face() == *block_side => {
                                    // Wall torch attached to this side of the solid block
                                    // A torch facing North is attached to a block to its South
                                    // So if we go North from the block, we find a torch facing North
                                    self.graph.add_edge(
                                        wire_node,
                                        component_node,
                                        CompileLink::new(LinkType::Default, distance),
                                    );
                                }
                                Block::RedstoneLamp { .. } | Block::IronTrapdoor { .. } | Block::NoteBlock { .. } => {
                                    self.graph.add_edge(
                                        wire_node,
                                        component_node,
                                        CompileLink::new(LinkType::Default, distance),
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                }

                // Handle diagonal wire connections
                if side.is_horizontal() {
                    if !up_block.is_solid() && !neighbor.is_transparent() {
                        let neighbor_up_pos = neighbor_pos.offset(BlockFace::Top);
                        if is_wire(self.world.get_block(neighbor_up_pos))
                            && !discovered.contains_key(&neighbor_up_pos)
                        {
                            if let Some(&neighbor_node) = self.pos_map.get(&neighbor_up_pos) {
                                self.graph.add_edge(
                                    wire_node,
                                    neighbor_node,
                                    CompileLink::new(LinkType::Default, distance + 1),
                                );
                                queue.push_back(neighbor_up_pos);
                                discovered.insert(neighbor_up_pos, distance + 1);
                            }
                        }
                    }

                    if !neighbor.is_solid() {
                        let neighbor_down_pos = neighbor_pos.offset(BlockFace::Bottom);
                        if is_wire(self.world.get_block(neighbor_down_pos))
                            && !discovered.contains_key(&neighbor_down_pos)
                        {
                            if let Some(&neighbor_node) = self.pos_map.get(&neighbor_down_pos) {
                                self.graph.add_edge(
                                    wire_node,
                                    neighbor_node,
                                    CompileLink::new(LinkType::Default, distance + 1),
                                );
                                queue.push_back(neighbor_down_pos);
                                discovered.insert(neighbor_down_pos, distance + 1);
                            }
                        }
                    }
                }
            }
        }
    }

    fn search_diode_inputs(&mut self, id: NodeIdx, pos: BlockPos, facing: BlockDirection) {
        let input_pos = pos.offset(facing.block_face());
        let input_block = self.block_lookup_cache.get_block(input_pos);
        self.get_redstone_links(
            input_block,
            facing.block_face(),
            input_pos,
            LinkType::Default,
            0,
            id,
            true,
        )
    }

    fn search_repeater_side(&mut self, id: NodeIdx, pos: BlockPos, side: BlockDirection) {
        let side_pos = pos.offset(side.block_face());
        let side_block = self.block_lookup_cache.get_block(side_pos);
        if mchprs_redstone::is_diode(side_block)
            && provides_weak_power(side_block, side.block_face())
        {
            self.graph
                .add_edge(self.pos_map[&side_pos], id, CompileLink::side(0));
        }
    }

    fn search_comparator_side(&mut self, id: NodeIdx, pos: BlockPos, side: BlockDirection) {
        let side_pos = pos.offset(side.block_face());
        let side_block = self.block_lookup_cache.get_block(side_pos);

        // Custom IO wires can have their power changed dynamically at runtime via
        // set_signal_strength(), so we must create a connection even if the wire
        // currently has power=0. This ensures comparators can read the runtime
        // power level from custom IO wires.
        let is_custom_io_wire =
            matches!(side_block, Block::RedstoneWire(_)) && self.custom_io.contains(&side_pos);

        if (mchprs_redstone::is_diode(side_block)
            && provides_weak_power(side_block, side.block_face()))
            || matches!(side_block, Block::RedstoneBlock)
            || is_custom_io_wire // Treat custom IO wires like redstone blocks for comparator sides
        {
            if let Some(&side_node) = self.pos_map.get(&side_pos) {
                self.graph
                    .add_edge(side_node, id, CompileLink::side(0));
            }
        } else if matches!(side_block, Block::RedstoneWire(_)) {
            self.search_wire(id, side_pos, LinkType::Side, 0)
        }
    }

    fn search_node(&mut self, id: NodeIdx, (pos, block_id): (BlockPos, u32)) {
        match Block::from_id(block_id) {
            Block::RedstoneTorch { .. } => {
                let bottom_pos = pos.offset(BlockFace::Bottom);
                let bottom_block = self.block_lookup_cache.get_block(bottom_pos);
                self.get_redstone_links(
                    bottom_block,
                    BlockFace::Top,
                    bottom_pos,
                    LinkType::Default,
                    0,
                    id,
                    true,
                );
            }
            Block::RedstoneWallTorch { facing, .. } => {
                let wall_pos = pos.offset(facing.opposite().block_face());
                let wall_block = self.block_lookup_cache.get_block(wall_pos);
                self.get_redstone_links(
                    wall_block,
                    facing.opposite().block_face(),
                    wall_pos,
                    LinkType::Default,
                    0,
                    id,
                    true,
                );
            }
            Block::Comparator(comparator) => {
                let facing = comparator.facing;

                self.search_comparator_side(id, pos, facing.rotate());
                self.search_comparator_side(id, pos, facing.rotate_ccw());

                let input_pos = pos.offset(facing.block_face());
                let input_block = self.block_lookup_cache.get_block(input_pos);
                if comparator::has_override(input_block) {
                    self.graph
                        .add_edge(self.pos_map[&input_pos], id, CompileLink::default(0));
                } else {
                    self.search_diode_inputs(id, pos, facing);
                }
            }
            Block::Repeater(repeater) => {
                let facing = repeater.facing;

                self.search_diode_inputs(id, pos, facing);
                self.search_repeater_side(id, pos, facing.rotate());
                self.search_repeater_side(id, pos, facing.rotate_ccw());
            }
            Block::RedstoneWire(_) => {
                // Custom IO wires are treated as normal wires
                // They participate in normal wire propagation via search_wire()
                // When set_signal_strength() is called at runtime, custom_io_override flag
                // prevents recalculation, making them act as sources
                let is_custom_io = self.custom_io.contains(&pos);
                self.search_wire(id, pos, LinkType::Default, 0);
                
                // BUGFIX: ALL wires connected to custom IO need output edges to propagate power!
                // Not just the custom IO wires themselves, but also any wire marked as input/output
                // (which happens when they're connected to custom IO wires)
                let node = &self.graph[id];
                let needs_output_edges = is_custom_io || node.is_input || node.is_output;
                
                if needs_output_edges {
                    self.create_wire_output_edges(id, pos);
                }
            }
            Block::RedstoneLamp { .. } | Block::IronTrapdoor { .. } | Block::NoteBlock { .. } => {
                for face in &BlockFace::values() {
                    let neighbor_pos = pos.offset(*face);
                    let neighbor_block = self.block_lookup_cache.get_block(neighbor_pos);
                    self.get_redstone_links(
                        neighbor_block,
                        *face,
                        neighbor_pos,
                        LinkType::Default,
                        0,
                        id,
                        true,
                    );
                }
            }
            Block::Observer { facing, .. } => {
                // The observer's only input is a trigger edge from the node it watches
                // (its "lens" face) — any propagation reaching this node means that node's
                // power/state changed, which is what schedules the pulse at simulation time.
                let watched_pos = pos.offset(facing.block_face());
                if let Some(&watched_node) = self.pos_map.get(&watched_pos) {
                    self.graph.add_edge(watched_node, id, CompileLink::default(0));
                }
            }
            Block::PoweredRail(rail) => {
                for face in &BlockFace::values() {
                    let neighbor_pos = pos.offset(*face);
                    let neighbor_block = self.block_lookup_cache.get_block(neighbor_pos);
                    self.get_redstone_links(
                        neighbor_block,
                        *face,
                        neighbor_pos,
                        LinkType::Default,
                        0,
                        id,
                        true,
                    );
                }
                self.search_rail_chain(id, pos, rail.shape, mchprs_redstone::rail::RailKind::Powered);
            }
            Block::ActivatorRail(rail) => {
                for face in &BlockFace::values() {
                    let neighbor_pos = pos.offset(*face);
                    let neighbor_block = self.block_lookup_cache.get_block(neighbor_pos);
                    self.get_redstone_links(
                        neighbor_block,
                        *face,
                        neighbor_pos,
                        LinkType::Default,
                        0,
                        id,
                        true,
                    );
                }
                self.search_rail_chain(id, pos, rail.shape, mchprs_redstone::rail::RailKind::Activator);
            }
            _ => {}
        }
    }

    /// Fans power-source edges from every rail in this rail's chain (up to 8 same-kind,
    /// axis-connected rails in each direction) into `id`, so `get_bool_input`'s existing
    /// "any input powered" logic implements the chain-relay rule at simulation time.
    fn search_rail_chain(
        &mut self,
        id: NodeIdx,
        pos: BlockPos,
        shape: StraightRailShape,
        kind: mchprs_redstone::rail::RailKind,
    ) {
        let (d1, d2) = mchprs_redstone::rail::shape_ends(shape);
        for dir in [d1, d2] {
            let mut chain_positions = Vec::new();
            mchprs_redstone::rail::walk_chain(self.world, pos, dir, kind, 8, |p| {
                chain_positions.push(p)
            });
            for chain_pos in chain_positions {
                for face in &BlockFace::values() {
                    let neighbor_pos = chain_pos.offset(*face);
                    let neighbor_block = self.block_lookup_cache.get_block(neighbor_pos);
                    self.get_redstone_links(
                        neighbor_block,
                        *face,
                        neighbor_pos,
                        LinkType::Default,
                        0,
                        id,
                        true,
                    );
                }
            }
        }
    }

    fn search(&mut self) {
        for i in 0..self.graph.node_bound() {
            let idx = NodeIdx::new(i);
            if !self.graph.contains_node(idx) {
                continue;
            }
            let node = &self.graph[idx];
            if let Some(block) = node.block {
                self.search_node(idx, block);
            }
        }
    }
}

fn is_wire(block: Block) -> bool {
    matches!(block, Block::RedstoneWire(_))
}

/// Returns `true` if the given block provides either weak or strong power to the given side.
/// Note that `side` is the side of the block receiving power, not the side of the block providing power.
/// Wires never match here: custom IO wires only act as power sources once
/// `set_signal_strength()` sets their `custom_io_override` flag at runtime.
fn provides_weak_power(block: Block, side: BlockFace) -> bool {
    if block.clone().get_pressure_plate_powered().is_some() {
        return true;
    }
    match block {
        Block::RedstoneTorch { .. } => side != BlockFace::Top,
        Block::RedstoneWallTorch { facing, .. } => facing.block_face() != side,
        Block::RedstoneBlock => true,
        Block::Lever { .. } => true,
        Block::StoneButton { .. } => true,
        Block::Repeater(repeater) => repeater.facing.block_face() == side,
        Block::Comparator(comparator) => comparator.facing.block_face() == side,
        // The observer's output is its back face — opposite the watched ("facing") side.
        Block::Observer { facing, .. } => facing.opposite().block_face() == side,
        _ => false,
    }
}

/// Returns `true` if the given block provides strong power to the given side.
/// Note that `side` is the side of the block receiving power, not the side of the block providing power.
fn provides_strong_power(block: Block, side: BlockFace) -> bool {
    if block.clone().get_pressure_plate_powered().is_some() && side == BlockFace::Top {
        return true;
    }
    match block {
        Block::RedstoneTorch { .. } if side == BlockFace::Bottom => true,
        Block::RedstoneWallTorch { .. } if side == BlockFace::Bottom => true,
        Block::Lever { face, facing, .. } => match side {
            BlockFace::Top => face == LeverFace::Floor,
            BlockFace::Bottom => face == LeverFace::Ceiling,
            _ => face == LeverFace::Wall && facing == side.unwrap_direction(),
        },
        Block::StoneButton { face, facing, .. } => match side {
            BlockFace::Top => face == LeverFace::Floor,
            BlockFace::Bottom => face == LeverFace::Ceiling,
            _ => face == LeverFace::Wall && facing == side.unwrap_direction(),
        },
        Block::Repeater(repeater) => repeater.facing.block_face() == side,
        Block::Comparator(comparator) => comparator.facing.block_face() == side,
        Block::Observer { facing, .. } => facing.opposite().block_face() == side,
        _ => false,
    }
}

