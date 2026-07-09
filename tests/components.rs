mod common;
use common::*;

use mchprs_blocks::blocks::Block;
use mchprs_blocks::BlockDirection;
use mchprs_world::{testing::TestWorld, TickPriority, World};

test_all_backends!(lever_on_off);
fn lever_on_off(backend: TestBackend) {
    let lever_pos = pos(0, 1, 0);

    let mut world = TestWorld::new(1, 1, 1);
    make_lever(&mut world, lever_pos);

    let mut runner = BackendRunner::new(world, backend);
    runner.check_block_powered(lever_pos, false);

    runner.use_block(lever_pos);
    runner.check_block_powered(lever_pos, true);

    runner.use_block(lever_pos);
    runner.check_block_powered(lever_pos, false);
}

test_all_backends!(trapdoor_on_off);
fn trapdoor_on_off(backend: TestBackend) {
    let lever_pos = pos(0, 1, 0);
    let trapdoor_pos = pos(1, 0, 0);

    let mut world = TestWorld::new(1, 1, 1);
    make_lever(&mut world, lever_pos);
    world.set_block(trapdoor_pos, trapdoor());

    let mut runner = BackendRunner::new(world, backend);
    runner.check_block_powered(trapdoor_pos, false);

    runner.use_block(lever_pos);
    runner.check_block_powered(trapdoor_pos, true);

    runner.use_block(lever_pos);
    runner.check_block_powered(trapdoor_pos, false);
}

test_all_backends!(lamp_on_off);
fn lamp_on_off(backend: TestBackend) {
    let lever_pos = pos(0, 1, 0);
    let lamp_pos = pos(1, 0, 0);

    let mut world = TestWorld::new(1, 1, 1);
    make_lever(&mut world, lever_pos);
    world.set_block(lamp_pos, Block::RedstoneLamp { lit: false });

    let mut runner = BackendRunner::new(world, backend);
    runner.check_block_powered(lamp_pos, false);

    runner.use_block(lever_pos);
    runner.check_block_powered(lamp_pos, true);

    runner.use_block(lever_pos);
    runner.check_powered_for(lamp_pos, true, 2);
    runner.check_block_powered(lamp_pos, false);
}

test_all_backends!(wall_torch_on_off);
fn wall_torch_on_off(backend: TestBackend) {
    let lever_pos = pos(0, 1, 0);
    let torch_pos = pos(1, 0, 0);

    let mut world = TestWorld::new(1, 1, 1);
    make_lever(&mut world, lever_pos);
    world.set_block(
        torch_pos,
        Block::RedstoneWallTorch {
            lit: true,
            facing: BlockDirection::East,
        },
    );

    let mut runner = BackendRunner::new(world, backend);
    runner.check_block_powered(torch_pos, true);

    runner.use_block(lever_pos);
    runner.check_powered_for(torch_pos, true, 1);
    runner.check_block_powered(torch_pos, false);

    runner.use_block(lever_pos);
    runner.check_powered_for(torch_pos, false, 1);
    runner.check_block_powered(torch_pos, true);
}

test_all_backends!(torch_on_off);
fn torch_on_off(backend: TestBackend) {
    let lever_pos = pos(0, 2, 0);
    let torch_pos = pos(2, 2, 0);

    let mut world = TestWorld::new(1, 1, 1);
    make_lever(&mut world, lever_pos);
    make_wire(&mut world, pos(1, 1, 0));
    place_on_block(&mut world, torch_pos, Block::RedstoneTorch { lit: true });

    let mut runner = BackendRunner::new(world, backend);
    runner.check_block_powered(torch_pos, true);

    runner.use_block(lever_pos);
    runner.check_powered_for(torch_pos, true, 1);
    runner.check_block_powered(torch_pos, false);

    runner.use_block(lever_pos);
    runner.check_powered_for(torch_pos, false, 1);
    runner.check_block_powered(torch_pos, true);
}

test_all_backends!(repeater_on_off);
fn repeater_on_off(backend: TestBackend) {
    let lever_pos = pos(0, 2, 0);
    let trapdoor_pos = pos(2, 1, 0);

    for delay in 1..=4 {
        let mut world = TestWorld::new(1, 1, 1);
        make_lever(&mut world, lever_pos);
        make_repeater(&mut world, pos(1, 1, 0), delay as u8, BlockDirection::West);
        world.set_block(trapdoor_pos, trapdoor());

        let mut runner = BackendRunner::new(world, backend);
        runner.check_block_powered(trapdoor_pos, false);

        // Check with a 1 tick pulse
        runner.use_block(lever_pos);
        runner.check_powered_for(trapdoor_pos, false, delay);
        runner.check_block_powered(trapdoor_pos, true);
        runner.use_block(lever_pos);
        runner.check_powered_for(trapdoor_pos, true, delay);
        runner.check_block_powered(trapdoor_pos, false);

        // Now a 0 tick pulse
        runner.use_block(lever_pos);
        runner.use_block(lever_pos);
        runner.check_powered_for(trapdoor_pos, false, delay);
        runner.check_powered_for(trapdoor_pos, true, delay);
        runner.check_block_powered(trapdoor_pos, false);
    }
}

test_all_backends!(wire_barely_reaches);
fn wire_barely_reaches(backend: TestBackend) {
    let lever_pos = pos(0, 1, 0);
    let trapdoor_pos = pos(16, 1, 0);

    let mut world = TestWorld::new(2, 1, 1);
    make_lever(&mut world, lever_pos);
    // 15 wire blocks between lever and trapdoor
    for x in 1..=15 {
        make_wire(&mut world, pos(x, 1, 0));
    }
    world.set_block(trapdoor_pos, trapdoor());

    let mut runner = BackendRunner::new(world, backend);
    runner.check_block_powered(trapdoor_pos, false);
    runner.use_block(lever_pos);
    runner.check_block_powered(trapdoor_pos, true);
    runner.use_block(lever_pos);
    runner.check_block_powered(trapdoor_pos, false);
}

test_all_backends!(wire_no_reach);
fn wire_no_reach(backend: TestBackend) {
    let lever_pos = pos(0, 1, 0);
    let trapdoor_pos = pos(17, 1, 0);

    let mut world = TestWorld::new(2, 1, 1);
    make_lever(&mut world, lever_pos);
    // 16 wire blocks between lever and trapdoor
    for x in 1..=16 {
        make_wire(&mut world, pos(x, 1, 0));
    }
    world.set_block(trapdoor_pos, trapdoor());

    let mut runner = BackendRunner::new(world, backend);
    runner.check_block_powered(trapdoor_pos, false);
    runner.use_block(lever_pos);
    runner.check_block_powered(trapdoor_pos, false);
    runner.use_block(lever_pos);
    runner.check_block_powered(trapdoor_pos, false);
}

test_all_backends!(ground_torch_does_not_power_block_below);
/// https://github.com/MCHPR/MCHPRS/issues/218
fn ground_torch_does_not_power_block_below(backend: TestBackend) {
    let torch_pos = pos(0, 1, 0);
    let lamp_pos = pos(0, 0, 0);

    let mut world = TestWorld::new(1, 1, 1);
    world.set_block(lamp_pos, Block::RedstoneLamp { lit: true });
    world.set_block(torch_pos, Block::RedstoneTorch { lit: true });

    world.schedule_tick(lamp_pos, 1, TickPriority::Normal);

    let mut runner = BackendRunner::new(world, backend);
    runner.tick();
    runner.check_block_powered(lamp_pos, false);
}

// Reproduction of an asymmetry observed downstream in Nucleation:
// a lever in the centre of two mirror-image wire arms toggles the torch
// on one end but not the other, even though the wires both reach signal
// strength 15 symmetrically.
//
// Layout (slice along z, looking down y=1 then y=2):
//
//   z=0           z=1           z=2           z=3           z=4
//   torch    --   wire    --   lever   --    wire    --   torch       (y=2)
//   ^host         ^host         ^host         ^host         ^host
//
// Both torches start lit; toggling the lever ON should turn BOTH off.
// Reproducer for downstream Nucleation failure: same circuit as
// `symmetric_torch_arms_must_match`, but compiled with
// `CompilerOptions { wire_dot_out: true, .. }` (which Nucleation passes).
#[test]
fn symmetric_torch_arms_must_match_wire_dot_out() {
    use mchprs_blocks::blocks::{LeverFace, RedstoneWire, RedstoneWireSide};
    use mchprs_blocks::BlockPos;
    use mchprs_redpiler::{BackendVariant, Compiler, CompilerOptions};

    let host_pos = pos(1, 1, 2);
    let lever_pos = pos(0, 1, 2);
    let wire_n_pos = pos(1, 1, 1);
    let wire_s_pos = pos(1, 1, 3);
    let torch_n_pos = pos(1, 2, 0);
    let torch_s_pos = pos(1, 2, 4);

    let mut world = TestWorld::new(1, 1, 1);
    world.set_block(host_pos, Block::Sandstone {});
    world.set_block(
        lever_pos,
        Block::Lever {
            face: LeverFace::Wall,
            facing: BlockDirection::West,
            powered: false,
        },
    );
    let ns_wire = RedstoneWire {
        north: RedstoneWireSide::Side,
        south: RedstoneWireSide::Side,
        east: RedstoneWireSide::None,
        west: RedstoneWireSide::None,
        power: 0,
    };
    place_on_block(&mut world, wire_n_pos, Block::RedstoneWire(ns_wire));
    place_on_block(&mut world, wire_s_pos, Block::RedstoneWire(ns_wire));
    place_on_block(&mut world, torch_n_pos, Block::RedstoneTorch { lit: true });
    place_on_block(&mut world, torch_s_pos, Block::RedstoneTorch { lit: true });

    let options = CompilerOptions {
        backend_variant: BackendVariant::Direct,
        wire_dot_out: true,
        ..Default::default()
    };
    let mut compiler = Compiler::default();
    let bounds = (BlockPos::new(0, 0, 0), BlockPos::new(15, 15, 15));
    let monitor = Default::default();
    compiler.compile(&world, bounds, options, vec![], monitor);

    compiler.on_use_block(lever_pos);
    compiler.tick();
    compiler.flush(&mut world);

    let lit = |p: BlockPos| matches!(world.get_block(p), Block::RedstoneTorch { lit: true });
    eprintln!(
        "wire_dot_out: torch_n.lit={} torch_s.lit={}",
        lit(torch_n_pos),
        lit(torch_s_pos)
    );
    assert_eq!(
        lit(torch_n_pos),
        lit(torch_s_pos),
        "torches at (1,2,0) and (1,2,4) should be in the same lit state"
    );
}

test_all_backends!(symmetric_torch_arms_must_match);
fn symmetric_torch_arms_must_match(backend: TestBackend) {
    use mchprs_blocks::blocks::{LeverFace, RedstoneWire, RedstoneWireSide};

    // Mirror image: wall lever on the WEST face of a central solid block,
    // wires fanning out N/S, torches on solid blocks at each end.
    //
    // Wires are placed with explicit N+S-only sides (no E/W) — this is the
    // shape Nucleation produces when the user writes
    // `state={"north": "side", "south": "side"}`. The redpiler's
    // wire-to-output edge construction is sensitive to this shape.
    let host_pos = pos(1, 1, 2);
    let lever_pos = pos(0, 1, 2);
    let wire_n_pos = pos(1, 1, 1);
    let wire_s_pos = pos(1, 1, 3);
    let torch_n_pos = pos(1, 2, 0);
    let torch_s_pos = pos(1, 2, 4);

    let mut world = TestWorld::new(1, 1, 1);
    world.set_block(host_pos, Block::Sandstone {});
    world.set_block(
        lever_pos,
        Block::Lever {
            face: LeverFace::Wall,
            facing: BlockDirection::West,
            powered: false,
        },
    );

    // North/south-only wires; sandstone underneath each.
    let ns_wire = RedstoneWire {
        north: RedstoneWireSide::Side,
        south: RedstoneWireSide::Side,
        east: RedstoneWireSide::None,
        west: RedstoneWireSide::None,
        power: 0,
    };
    place_on_block(&mut world, wire_n_pos, Block::RedstoneWire(ns_wire));
    place_on_block(&mut world, wire_s_pos, Block::RedstoneWire(ns_wire));

    place_on_block(&mut world, torch_n_pos, Block::RedstoneTorch { lit: true });
    place_on_block(&mut world, torch_s_pos, Block::RedstoneTorch { lit: true });

    let mut runner = BackendRunner::new(world, backend);
    runner.check_block_powered(torch_n_pos, true);
    runner.check_block_powered(torch_s_pos, true);

    // Toggle lever ON; both torches should turn off after one tick.
    runner.use_block(lever_pos);
    runner.tick();
    runner.check_block_powered(torch_n_pos, false);
    runner.check_block_powered(torch_s_pos, false);
}

// Two mirror-image repeaters fed by the same powered block must keep
// their original `facing` after flush — Coalesce merges them into one
// node (same delay, same single incoming edge), and a naive flush would
// stamp the survivor's whole Block (including facing) over the alias's
// world position.
test_all_backends!(symmetric_repeaters_keep_facing);
fn symmetric_repeaters_keep_facing(backend: TestBackend) {
    use mchprs_blocks::blocks::{LeverFace, Repeater};

    let host_pos = pos(1, 1, 2);
    let lever_pos = pos(0, 1, 2);
    let rep_n_pos = pos(1, 1, 1);
    let rep_s_pos = pos(1, 1, 3);

    let mut world = TestWorld::new(1, 1, 1);
    world.set_block(host_pos, Block::Sandstone {});
    world.set_block(
        lever_pos,
        Block::Lever {
            face: LeverFace::Wall,
            facing: BlockDirection::West,
            powered: false,
        },
    );
    // In MCHPRS, a repeater's `facing` points at its INPUT side (the
    // block whose power it reads). Mirror image: rep_n is north of the
    // host so it must face South to read the host; rep_s is south of
    // the host so it must face North to read the host. Both repeaters
    // share the same NodeType (delay=1, facing_diode=false), so the
    // Coalesce pass merges them — the regression we want to catch is
    // an alias's facing getting clobbered by the survivor's.
    place_on_block(
        &mut world,
        rep_n_pos,
        Block::Repeater(Repeater {
            delay: 1,
            facing: BlockDirection::South,
            ..Default::default()
        }),
    );
    place_on_block(
        &mut world,
        rep_s_pos,
        Block::Repeater(Repeater {
            delay: 1,
            facing: BlockDirection::North,
            ..Default::default()
        }),
    );

    let mut runner = BackendRunner::new(world, backend);
    runner.use_block(lever_pos);
    runner.tick();
    runner.tick();

    runner.check_block_powered(rep_n_pos, true);
    runner.check_block_powered(rep_s_pos, true);

    // Critical assertion: each repeater kept its own facing.
    let n_facing = match runner.get_block(rep_n_pos) {
        Block::Repeater(repeater) => repeater.facing,
        b => panic!("expected repeater at {:?}, got {:?}", rep_n_pos, b),
    };
    let s_facing = match runner.get_block(rep_s_pos) {
        Block::Repeater(repeater) => repeater.facing,
        b => panic!("expected repeater at {:?}, got {:?}", rep_s_pos, b),
    };
    assert_eq!(n_facing, BlockDirection::South, "north repeater's facing was overwritten");
    assert_eq!(s_facing, BlockDirection::North, "south repeater's facing was overwritten");
}

// Wire that visually points only east/west must NOT power a solid block
// to its north or south. The redpiler's input_search previously walked
// every wire adjacent to a solid block regardless of visual side, which
// let a lever-driven E/W wire incorrectly power a north-side neighbour
// (and through it a repeater rear-input).
test_all_backends!(wire_does_not_power_perpendicular_solid);
fn wire_does_not_power_perpendicular_solid(backend: TestBackend) {
    use mchprs_blocks::blocks::{Repeater, RedstoneWire, RedstoneWireSide};

    let lever_pos = pos(0, 1, 0);
    // E/W-only wire at (1, 1, 0); lever sits to its west.
    let wire_pos  = pos(1, 1, 0);
    // Solid block to the wire's NORTH (i.e. -z = pos(1, 1, -1)? No,
    // we're inside a single chunk so pick z=1 = SOUTH instead). The
    // wire's south side is None after regulation, so this block is
    // perpendicular to the wire's visual run.
    let perp_solid_pos = pos(1, 1, 1);
    // Repeater whose rear sits on top of perp_solid: facing=North means
    // input direction is north → reads from (1, 1, 1).
    let repeater_pos   = pos(1, 1, 2);

    let mut world = TestWorld::new(1, 1, 1);
    make_lever(&mut world, lever_pos);

    let ew_wire = RedstoneWire {
        north: RedstoneWireSide::None,
        south: RedstoneWireSide::None,
        east:  RedstoneWireSide::Side,
        west:  RedstoneWireSide::Side,
        power: 0,
    };
    place_on_block(&mut world, wire_pos, Block::RedstoneWire(ew_wire));
    // Solid host for the perpendicular block (place_on_block handles it).
    place_on_block(&mut world, perp_solid_pos, Block::Sandstone {});
    place_on_block(
        &mut world,
        repeater_pos,
        Block::Repeater(Repeater {
            delay: 1,
            facing: BlockDirection::North,
            ..Default::default()
        }),
    );

    let mut runner = BackendRunner::new(world, backend);
    runner.use_block(lever_pos);
    runner.tick();
    runner.tick();
    runner.check_block_powered(repeater_pos, false);
}
