mod common;
use common::*;

use mchprs_blocks::blocks::{Block, Comparator, ComparatorMode, Repeater};
use mchprs_blocks::{BlockDirection, BlockPos};
use mchprs_redpiler::{BackendVariant, Compiler, CompilerOptions};
use mchprs_world::{testing::TestWorld, World};

/// BUG: Custom IO redstone wire doesn't propagate power to adjacent wires
#[test]
fn custom_io_wire_doesnt_power_adjacent_wire() {
    let custom_io_pos = pos(0, 0, 0);
    let normal_wire_pos = pos(1, 0, 0);

    let mut world = TestWorld::new(1, 1, 1);
    
    // Two adjacent wires: custom IO at X=0, normal at X=1
    make_wire(&mut world, custom_io_pos);
    make_wire(&mut world, normal_wire_pos);

    let options = CompilerOptions {
        backend_variant: BackendVariant::Direct,
        io_only: false,
        custom_io: vec![custom_io_pos],
        ..Default::default()
    };
    
    let mut compiler = Compiler::default();
    let bounds = (BlockPos::new(0, 0, 0), BlockPos::new(15, 15, 15));
    let monitor = Default::default();
    let ticks = vec![];
    
    compiler.compile(&world, bounds, options, ticks, monitor);
    
    // Set custom IO wire to power 15
    compiler.set_signal_strength(custom_io_pos, 15);
    compiler.flush(&mut world);
    
    // Tick to let signal propagate
    compiler.tick();
    compiler.flush(&mut world);
    
    // Check powers
    let custom_io_power = compiler.get_signal_strength(custom_io_pos).unwrap();
    let normal_wire_power = compiler.get_signal_strength(normal_wire_pos).unwrap();
    
    println!("Custom IO wire power: {}", custom_io_power);
    println!("Normal wire power: {}", normal_wire_power);
    
    // BUG: Normal wire should have power 14 (15 - 1 for distance), but it has 0
    assert_eq!(custom_io_power, 15, "Custom IO should be at power 15");
    assert_eq!(normal_wire_power, 14, "Adjacent wire should receive power 14 (BUG: it stays at 0)");
}

/// BUG: Custom IO redstone wire adjacent to a solid block doesn't power components on top
#[test]
fn custom_io_adjacent_to_block_doesnt_power_torch() {
    let wire_pos = pos(0, 0, 0);
    let block_pos = pos(0, 1, 0);
    let torch_pos = pos(0, 2, 0);

    let mut world = TestWorld::new(1, 1, 1);
    
    // Setup: wire -> block -> torch (vertically stacked)
    make_wire(&mut world, wire_pos);
    world.set_block(block_pos, Block::GrayConcrete);
    world.set_block(torch_pos, Block::RedstoneTorch { lit: true });

    let options = CompilerOptions {
        backend_variant: BackendVariant::Direct,
        io_only: false,
        custom_io: vec![wire_pos],
        ..Default::default()
    };
    
    let mut compiler = Compiler::default();
    let bounds = (BlockPos::new(0, 0, 0), BlockPos::new(15, 15, 15));
    let monitor = Default::default();
    let ticks = vec![];
    
    compiler.compile(&world, bounds, options, ticks, monitor);
    
    // Set custom IO wire to power 15
    compiler.set_signal_strength(wire_pos, 15);
    compiler.flush(&mut world);
    
    // Tick to let signal propagate
    compiler.tick();
    compiler.flush(&mut world);
    compiler.tick();
    compiler.flush(&mut world);
    
    let final_torch = world.get_block(torch_pos);
    println!("Torch state: {:?}", final_torch);
    
    // BUG: Torch should turn OFF when the block below is powered, but it stays ON
    assert_eq!(
        final_torch,
        Block::RedstoneTorch { lit: false },
        "Torch should be OFF when block below is powered (BUG: it stays ON)"
    );
}

/// Test that comparators correctly read side input power from custom IO wires
/// This was previously a bug where custom IO wires weren't properly connected to comparator
/// side inputs during compilation, causing the comparator to ignore dynamic power changes.
#[test]
fn comparator_subtract_ignores_custom_io_side_input() {
    let back_input_pos = pos(0, 1, 2);
    let comparator_pos = pos(0, 1, 1);
    let side_input_pos = pos(1, 1, 1);
    let output_pos = pos(0, 1, 0);

    let mut world = TestWorld::new(1, 1, 1);
    
    // Base layer (Y=0)
    world.set_block(pos(0, 0, 0), Block::GrayConcrete);
    world.set_block(pos(1, 0, 0), Block::GrayConcrete);
    world.set_block(pos(0, 0, 1), Block::GrayConcrete);
    world.set_block(pos(1, 0, 1), Block::GrayConcrete);
    world.set_block(pos(0, 0, 2), Block::GrayConcrete);
    world.set_block(pos(1, 0, 2), Block::GrayConcrete);
    
    // Circuit: back_input -> comparator <- side_input  (Y=1)
    //                           ↓
    //                         output
    make_wire(&mut world, back_input_pos);
    world.set_block(
        comparator_pos,
        Block::Comparator(Comparator {
            facing: BlockDirection::South,
            mode: ComparatorMode::Subtract,
            powered: false,
        }),
    );
    make_wire(&mut world, side_input_pos);
    make_wire(&mut world, output_pos);

    let options = CompilerOptions {
        backend_variant: BackendVariant::Direct,
        io_only: false,
        custom_io: vec![back_input_pos, side_input_pos, output_pos],
        ..Default::default()
    };
    
    let mut compiler = Compiler::default();
    let bounds = (BlockPos::new(0, 0, 0), BlockPos::new(15, 15, 15));
    let monitor = Default::default();
    let ticks = vec![];
    
    compiler.compile(&world, bounds, options, ticks, monitor);
    
    println!("\n=== Test 1: back=ON, side=OFF (should output ON) ===");
    compiler.set_signal_strength(back_input_pos, 15);
    compiler.set_signal_strength(side_input_pos, 0);
    compiler.flush(&mut world);  // Sync custom IO to world BEFORE ticking
    compiler.tick();
    compiler.flush(&mut world);
    
    let output1 = compiler.get_signal_strength(output_pos).unwrap();
    println!("back=15, side=0 → output={} (expected 15)", output1);
    assert_eq!(output1, 15, "Inverter case: back ON, side OFF should output ON");
    
    println!("\n=== Test 2: back=ON, side=ON (should output OFF - BUG) ===");
    compiler.set_signal_strength(back_input_pos, 15);
    compiler.set_signal_strength(side_input_pos, 15);
    compiler.flush(&mut world);  // Sync custom IO to world BEFORE ticking
    compiler.tick();
    compiler.flush(&mut world);
    
    let output2 = compiler.get_signal_strength(output_pos).unwrap();
    println!("back=15, side=15 → output={} (expected 0)", output2);
    
    // Comparator in subtract mode should compute max(rear - side, 0)
    // With rear=15, side=15: output should be max(15-15, 0) = 0
    assert_eq!(
        output2, 0,
        "Comparator subtract mode with custom IO: Output={}, expected 0",
        output2
    );
}

/// CONTROL TEST: Comparator with repeater between custom IO and side input works correctly
/// This confirms that the issue is specific to custom IO wires being read directly by the comparator
#[test]
fn comparator_subtract_with_repeater_works() {
    let back_input_pos = pos(0, 1, 2);
    let comparator_pos = pos(0, 1, 1);
    let repeater_pos = pos(1, 1, 1);
    let side_input_pos = pos(2, 1, 1);
    let output_pos = pos(0, 1, 0);

    let mut world = TestWorld::new(1, 1, 1);
    
    // Base layer (Y=0)
    for x in 0..3 {
        for z in 0..3 {
            world.set_block(pos(x, 0, z), Block::GrayConcrete);
        }
    }
    
    // Circuit: side_input -> repeater -> comparator <- back_input  (Y=1)
    //                                        ↓
    //                                      output
    make_wire(&mut world, back_input_pos);
    world.set_block(
        comparator_pos,
        Block::Comparator(Comparator {
            facing: BlockDirection::South,
            mode: ComparatorMode::Subtract,
            powered: false,
        }),
    );
    world.set_block(
        repeater_pos,
        Block::Repeater(Repeater {
            delay: 1,
            facing: BlockDirection::East,
            locked: false,
            powered: false,
        }),
    );
    make_wire(&mut world, side_input_pos);
    make_wire(&mut world, output_pos);

    let options = CompilerOptions {
        backend_variant: BackendVariant::Direct,
        io_only: false,
        custom_io: vec![back_input_pos, side_input_pos, output_pos],
        ..Default::default()
    };
    
    let mut compiler = Compiler::default();
    let bounds = (BlockPos::new(0, 0, 0), BlockPos::new(15, 15, 15));
    let monitor = Default::default();
    let ticks = vec![];
    
    compiler.compile(&world, bounds, options, ticks, monitor);
    
    println!("\n=== WITH REPEATER: back=ON, side=ON (should output OFF) ===");
    compiler.set_signal_strength(back_input_pos, 15);
    compiler.set_signal_strength(side_input_pos, 15);
    
    // Extra ticks for repeater delay
    for _ in 0..10 {
        compiler.tick();
        compiler.flush(&mut world);
    }
    
    let output = compiler.get_signal_strength(output_pos).unwrap();
    println!("back=15, side=15 → output={} (expected 0)", output);
    
    // With a repeater, the comparator should work correctly!
    // The repeater reads from the custom IO wire and outputs to a non-custom-IO position
    // So the comparator can read the side input correctly
    assert_eq!(
        output, 0,
        "WITH REPEATER: Comparator should correctly subtract. Output={}, expected 0",
        output
    );
}


