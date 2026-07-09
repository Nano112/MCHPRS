mod common;
use common::*;

use mchprs_blocks::blocks::{Block, RailShape, StraightRailShape};
use mchprs_blocks::BlockDirection;
use mchprs_redstone::rail;
use mchprs_world::testing::TestWorld;
use mchprs_world::World;

test_all_backends!(powered_rail_direct_power);
fn powered_rail_direct_power(backend: TestBackend) {
    let lever_pos = pos(0, 1, 0);
    let rail_pos = pos(1, 1, 0);

    let mut world = TestWorld::new(1, 1, 1);
    make_lever(&mut world, lever_pos);
    make_powered_rail(&mut world, rail_pos, StraightRailShape::EastWest);

    let mut runner = BackendRunner::new(world, backend);
    runner.check_block_powered(rail_pos, false);
    runner.use_block(lever_pos);
    runner.check_powered_for(rail_pos, true, 2);
}

test_all_backends!(activator_rail_direct_power);
fn activator_rail_direct_power(backend: TestBackend) {
    let lever_pos = pos(0, 1, 0);
    let rail_pos = pos(1, 1, 0);

    let mut world = TestWorld::new(1, 1, 1);
    make_lever(&mut world, lever_pos);
    make_activator_rail(&mut world, rail_pos, StraightRailShape::EastWest);

    let mut runner = BackendRunner::new(world, backend);
    runner.check_block_powered(rail_pos, false);
    runner.use_block(lever_pos);
    runner.check_powered_for(rail_pos, true, 2);
}

test_all_backends!(powered_rail_chain_reaches_eight);
fn powered_rail_chain_reaches_eight(backend: TestBackend) {
    let lever_pos = pos(0, 1, 0);

    let mut world = TestWorld::new(1, 1, 1);
    make_lever(&mut world, lever_pos);
    // Rail at x=1 sits directly against the lever; the rail at x=9 is 8 rails away from it.
    for x in 1..=9 {
        make_powered_rail(&mut world, pos(x, 1, 0), StraightRailShape::EastWest);
    }

    let mut runner = BackendRunner::new(world, backend);
    runner.use_block(lever_pos);
    runner.check_powered_for(pos(1, 1, 0), true, 1);
    runner.check_powered_for(pos(9, 1, 0), true, 1);
}

test_all_backends!(powered_rail_chain_does_not_reach_nine);
fn powered_rail_chain_does_not_reach_nine(backend: TestBackend) {
    let lever_pos = pos(0, 1, 0);

    let mut world = TestWorld::new(1, 1, 1);
    make_lever(&mut world, lever_pos);
    // Rail at x=10 is 9 rails away from the directly-powered rail at x=1: out of range.
    for x in 1..=10 {
        make_powered_rail(&mut world, pos(x, 1, 0), StraightRailShape::EastWest);
    }

    let mut runner = BackendRunner::new(world, backend);
    runner.use_block(lever_pos);
    runner.check_powered_for(pos(9, 1, 0), true, 1);
    runner.check_powered_for(pos(10, 1, 0), false, 1);
}

test_all_backends!(activator_rail_chain_does_not_relay_through_powered_rail);
fn activator_rail_chain_does_not_relay_through_powered_rail(backend: TestBackend) {
    let lever_pos = pos(0, 1, 0);

    let mut world = TestWorld::new(1, 1, 1);
    make_lever(&mut world, lever_pos);
    make_activator_rail(&mut world, pos(1, 1, 0), StraightRailShape::EastWest);
    // A powered rail in the middle of the line shouldn't relay the activator rail chain.
    make_powered_rail(&mut world, pos(2, 1, 0), StraightRailShape::EastWest);
    make_activator_rail(&mut world, pos(3, 1, 0), StraightRailShape::EastWest);

    let mut runner = BackendRunner::new(world, backend);
    runner.use_block(lever_pos);
    runner.check_powered_for(pos(1, 1, 0), true, 1);
    runner.check_powered_for(pos(3, 1, 0), false, 1);
}

#[test]
fn rail_shape_straight_and_curve_placement() {
    let mut world = TestWorld::new(1, 1, 1);

    // A rail with neighbors only to the west and east should end up straight (east-west).
    world.set_block(pos(0, 0, 1), Block::Sandstone {});
    world.set_block(pos(1, 0, 1), Block::Sandstone {});
    world.set_block(pos(2, 0, 1), Block::Sandstone {});
    world.set_block(
        pos(0, 1, 1),
        Block::Rail(rail::rail_get_state_for_placement(
            &world,
            pos(0, 1, 1),
            BlockDirection::East,
        )),
    );
    world.set_block(
        pos(2, 1, 1),
        Block::Rail(rail::rail_get_state_for_placement(
            &world,
            pos(2, 1, 1),
            BlockDirection::East,
        )),
    );
    let middle = rail::rail_get_state_for_placement(&world, pos(1, 1, 1), BlockDirection::North);
    assert_eq!(middle.shape, RailShape::EastWest);

    // A rail with neighbors to the west and south should curve to connect them.
    let mut world = TestWorld::new(1, 1, 1);
    world.set_block(pos(4, 0, 5), Block::Sandstone {});
    world.set_block(pos(5, 0, 5), Block::Sandstone {});
    world.set_block(pos(5, 0, 6), Block::Sandstone {});
    world.set_block(
        pos(4, 1, 5),
        Block::Rail(rail::rail_get_state_for_placement(
            &world,
            pos(4, 1, 5),
            BlockDirection::East,
        )),
    );
    world.set_block(
        pos(5, 1, 6),
        Block::Rail(rail::rail_get_state_for_placement(
            &world,
            pos(5, 1, 6),
            BlockDirection::North,
        )),
    );
    let curve = rail::rail_get_state_for_placement(&world, pos(5, 1, 5), BlockDirection::North);
    assert_eq!(curve.shape, RailShape::SouthWest);
}

#[test]
fn rail_shape_ascending() {
    let mut world = TestWorld::new(1, 1, 1);
    // A solid block one higher to the west, with a rail on top of it, should make this rail
    // ascend to the west.
    world.set_block(pos(0, 0, 0), Block::Sandstone {});
    world.set_block(pos(0, 1, 0), Block::Sandstone {});
    world.set_block(pos(1, 0, 0), Block::Sandstone {});
    world.set_block(
        pos(0, 2, 0),
        Block::Rail(rail::rail_get_state_for_placement(
            &world,
            pos(0, 2, 0),
            BlockDirection::West,
        )),
    );
    let ascending = rail::rail_get_state_for_placement(&world, pos(1, 1, 0), BlockDirection::West);
    assert_eq!(ascending.shape, RailShape::AscendingWest);
}
