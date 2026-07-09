mod common;
use common::*;

use mchprs_blocks::BlockPos;
use mchprs_redpiler::{Compiler, CompilerOptions};
use mchprs_world::testing::TestWorld;
use mchprs_world::World;

#[test]
fn compile_graph_matches_simulated_node_count() {
    let lever_pos = pos(0, 1, 0);
    let repeater_pos = pos(1, 1, 0);
    let trapdoor_pos = pos(2, 1, 0);

    let mut world = TestWorld::new(1, 1, 1);
    make_lever(&mut world, lever_pos);
    make_repeater(&mut world, repeater_pos, 1, mchprs_blocks::BlockDirection::West);
    world.set_block(trapdoor_pos, trapdoor());

    let bounds = (BlockPos::new(0, 0, 0), BlockPos::new(15, 15, 15));
    let options = CompilerOptions {
        optimize: true,
        ..Default::default()
    };
    let nodes = Compiler::compile_graph(&world, bounds, options, Default::default());

    // Lever, repeater, trapdoor: three distinct components should survive
    // even the optimized (post-fold/coalesce) graph.
    assert_eq!(nodes.len(), 3, "expected 3 nodes, got {:#?}", nodes);
}

#[test]
fn compile_graph_structural_keeps_wires_unfolded() {
    let lever_pos = pos(0, 1, 0);
    let trapdoor_pos = pos(3, 1, 0);

    let mut world = TestWorld::new(1, 1, 1);
    make_lever(&mut world, lever_pos);
    make_wire(&mut world, pos(1, 1, 0));
    make_wire(&mut world, pos(2, 1, 0));
    world.set_block(trapdoor_pos, trapdoor());

    let bounds = (BlockPos::new(0, 0, 0), BlockPos::new(15, 15, 15));

    // Structural pass stops before ConstantFold/Coalesce: every component,
    // including each wire, should remain its own node.
    let structural_nodes = Compiler::compile_graph_structural(
        &world,
        bounds,
        CompilerOptions::default(),
        Default::default(),
    );
    assert_eq!(
        structural_nodes.len(),
        4,
        "expected lever + 2 wires + trapdoor = 4 structural nodes, got {:#?}",
        structural_nodes
    );

    // The optimized graph is free to fold/coalesce the wires away.
    let optimized_nodes = Compiler::compile_graph(
        &world,
        bounds,
        CompilerOptions {
            optimize: true,
            ..Default::default()
        },
        Default::default(),
    );
    assert!(
        optimized_nodes.len() < structural_nodes.len(),
        "optimized graph ({}) should be smaller than the structural graph ({})",
        optimized_nodes.len(),
        structural_nodes.len()
    );
}
