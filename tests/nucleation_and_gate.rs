mod common;
use common::*;

use mchprs_blocks::blocks::Block;
use mchprs_blocks::{BlockPos, BlockDirection};
use mchprs_redpiler::{BackendVariant, Compiler, CompilerOptions};
use mchprs_world::{testing::TestWorld, World};

/// Test that mirrors the Nucleation AND gate setup exactly
#[test]
fn nucleation_and_gate_both_true() {
    let mut world = TestWorld::new(1, 1, 1);

    // Create the AND gate circuit exactly as in Nucleation
    // Y=1 layer (ground)
    make_wire(&mut world, pos(0, 1, 0)); // Input A (custom IO)
    make_wire(&mut world, pos(1, 1, 0));
    world.set_block(pos(2, 1, 0), Block::GrayConcrete);

    make_wire(&mut world, pos(0, 1, 2)); // Input B (custom IO)
    make_wire(&mut world, pos(1, 1, 2));
    world.set_block(pos(2, 1, 2), Block::GrayConcrete);
    
    world.set_block(pos(3, 1, 1), Block::RedstoneWallTorch {
        lit: false,  // Starts false, will turn on when both torches are off
        facing: BlockDirection::East,
    });
    make_wire(&mut world, pos(4, 1, 1)); // Output (custom IO)
    
    // Y=2 layer (torches)
    world.set_block(pos(2, 2, 0), Block::RedstoneTorch { lit: true });
    world.set_block(pos(2, 2, 2), Block::RedstoneTorch { lit: true });
    make_wire(&mut world, pos(2, 2, 1));
    
    // Custom IO positions
    let input_a = pos(0, 1, 0);
    let input_b = pos(0, 1, 2);
    let output = pos(4, 1, 1);
    
    let options = CompilerOptions {
        backend_variant: BackendVariant::Direct,
        io_only: false,
        custom_io: vec![input_a, input_b], // Don't mark output as custom IO to see if it gets powered
        ..Default::default()
    };
    
    let mut compiler = Compiler::default();
    let bounds = (BlockPos::new(0, 0, 0), BlockPos::new(15, 15, 15));
    let monitor = Default::default();
    let ticks = vec![];
    
    compiler.compile(&world, bounds, options, ticks, monitor);
    
    // Set both inputs to true (power 15)
    compiler.set_signal_strength(input_a, 15);
    compiler.set_signal_strength(input_b, 15);
    compiler.flush(&mut world);
    
    // Tick to let signal propagate
    compiler.tick();
    compiler.flush(&mut world);
    compiler.tick();
    compiler.flush(&mut world);
    
    // Check intermediate wires
    let wire_1_0 = compiler.get_signal_strength(pos(1, 1, 0));
    let wire_1_2 = compiler.get_signal_strength(pos(1, 1, 2));
    
    println!("Input A (0,1,0): {:?}", compiler.get_signal_strength(input_a));
    println!("Wire (1,1,0): {:?}", wire_1_0);
    println!("Input B (0,1,2): {:?}", compiler.get_signal_strength(input_b));
    println!("Wire (1,1,2): {:?}", wire_1_2);
    
    // Check torch states
    let torch_0 = world.get_block(pos(2, 2, 0));
    let torch_2 = world.get_block(pos(2, 2, 2));
    println!("Torch at (2,2,0): {:?}", torch_0);
    println!("Torch at (2,2,2): {:?}", torch_2);
    
    // Check intermediate wire
    let wire_2_2_1 = compiler.get_signal_strength(pos(2, 2, 1));
    println!("Wire at (2,2,1): {:?}", wire_2_2_1);
    
    // Check wall torch
    let wall_torch = world.get_block(pos(3, 1, 1));
    println!("Wall torch at (3,1,1): {:?}", wall_torch);
    
    // Check if wall torch has a node
    let wall_torch_power = compiler.get_signal_strength(pos(3, 1, 1));
    println!("Wall torch power at (3,1,1): {:?}", wall_torch_power);
    
    // Check output
    let output_power = compiler.get_signal_strength(output);
    println!("Output (4,1,1): {:?}", output_power);
    
    // Assertions
    assert_eq!(wire_1_0, Some(14), "Wire at (1,1,0) should have power 14");
    assert_eq!(wire_1_2, Some(14), "Wire at (1,1,2) should have power 14");
    assert_eq!(torch_0, Block::RedstoneTorch { lit: false }, "Torch at (2,2,0) should be OFF");
    assert_eq!(torch_2, Block::RedstoneTorch { lit: false }, "Torch at (2,2,2) should be OFF");
    assert_eq!(output_power, Some(15), "Output should have power 15 (true AND true = true)");
}

