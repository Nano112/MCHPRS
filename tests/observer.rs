mod common;
use common::*;

use mchprs_blocks::BlockFacing;
use mchprs_world::testing::TestWorld;

test_all_backends!(observer_pulse_timing);
fn observer_pulse_timing(backend: TestBackend) {
    let lever_pos = pos(0, 1, 0);
    let wire_pos = pos(1, 1, 0);
    let observer_pos = pos(2, 1, 0);

    let mut world = TestWorld::new(1, 1, 1);
    make_lever(&mut world, lever_pos);
    make_wire(&mut world, wire_pos);
    // Facing West: watches the wire at x=1, outputs from its east (back) face.
    make_observer(&mut world, observer_pos, BlockFacing::West);

    let mut runner = BackendRunner::new(world, backend);
    runner.check_block_powered(observer_pos, false);

    runner.use_block(lever_pos);
    // 2-game-tick delay before the pulse starts...
    runner.check_powered_for(observer_pos, false, 2);
    // ...then a 2-game-tick pulse...
    runner.check_powered_for(observer_pos, true, 2);
    // ...then back off, and it doesn't fire again on its own.
    runner.check_powered_for(observer_pos, false, 4);
}

test_all_backends!(observer_does_not_retrigger_mid_pulse);
fn observer_does_not_retrigger_mid_pulse(backend: TestBackend) {
    let lever_pos = pos(0, 1, 0);
    let wire_pos = pos(1, 1, 0);
    let observer_pos = pos(2, 1, 0);

    let mut world = TestWorld::new(1, 1, 1);
    make_lever(&mut world, lever_pos);
    make_wire(&mut world, wire_pos);
    make_observer(&mut world, observer_pos, BlockFacing::West);

    let mut runner = BackendRunner::new(world, backend);

    runner.use_block(lever_pos);
    runner.check_powered_for(observer_pos, false, 2);
    // Toggle the watched wire again while the observer is still waiting to fire; this must
    // not restart or extend the pulse that's already scheduled.
    runner.use_block(lever_pos);
    runner.use_block(lever_pos);
    runner.check_powered_for(observer_pos, true, 2);
    runner.check_block_powered(observer_pos, false);
}
