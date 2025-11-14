mod common;
use common::*;

use mchprs_blocks::blocks::Block;
use mchprs_blocks::{BlockColorVariant, BlockPos};
use mchprs_redpiler::{BackendVariant, Compiler, CompilerOptions};
use mchprs_world::World;

/// BUG: Custom IO redstone wire doesn't propagate power to adjacent wires
#[test]
fn custom_io_wire_doesnt_power_adjacent_wire() {
    let custom_io_pos = pos(0, 0, 0);
    let normal_wire_pos = pos(1, 0, 0);

    let mut world = TestWorld::new(1);
    
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

    let mut world = TestWorld::new(1);
    
    // Setup: wire -> block -> torch (vertically stacked)
    make_wire(&mut world, wire_pos);
    world.set_block(block_pos, Block::Concrete { color: BlockColorVariant::Gray });
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


