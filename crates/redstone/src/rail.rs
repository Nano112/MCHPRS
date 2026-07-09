use mchprs_blocks::blocks::{
    ActivatorRail, Block, DetectorRail, PoweredRail, Rail, RailShape, StraightRailShape,
};
use mchprs_blocks::{BlockDirection, BlockFace, BlockPos};
use mchprs_world::World;

/// Which redstone-relay chain family a rail belongs to. Powered rails only relay power
/// through other powered rails, activator rails only through other activator rails.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RailKind {
    Powered,
    Activator,
}

pub fn is_rail(block: Block) -> bool {
    matches!(
        block,
        Block::Rail(_) | Block::PoweredRail(_) | Block::ActivatorRail(_) | Block::DetectorRail(_)
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Connection {
    None,
    Flat,
    /// The neighbor in this direction is one block higher: this rail should ascend toward it.
    Up,
}

fn connection(world: &impl World, pos: BlockPos, direction: BlockDirection) -> Connection {
    let flat_pos = pos.offset(direction.block_face());
    if is_rail(world.get_block(flat_pos)) {
        return Connection::Flat;
    }
    let up_pos = flat_pos.offset(BlockFace::Top);
    if is_rail(world.get_block(up_pos)) {
        return Connection::Up;
    }
    Connection::None
}

fn axis_choice(
    north: Connection,
    south: Connection,
    east: Connection,
    west: Connection,
    fallback: BlockDirection,
) -> bool {
    let ns_connected = north != Connection::None || south != Connection::None;
    let ew_connected = east != Connection::None || west != Connection::None;
    match (ns_connected, ew_connected) {
        (true, false) => true,
        (false, true) => false,
        // Ambiguous (both, or neither): a straight rail can't express both, so keep
        // whichever axis the fallback (player facing on placement, current shape's axis
        // on reshape) points along.
        _ => matches!(fallback, BlockDirection::North | BlockDirection::South),
    }
}

/// Shape for a straight-only rail (powered/activator/detector): flat or ascending, never
/// a curve.
fn straight_shape(world: &impl World, pos: BlockPos, fallback: BlockDirection) -> StraightRailShape {
    let north = connection(world, pos, BlockDirection::North);
    let south = connection(world, pos, BlockDirection::South);
    let east = connection(world, pos, BlockDirection::East);
    let west = connection(world, pos, BlockDirection::West);

    if axis_choice(north, south, east, west, fallback) {
        match (north, south) {
            (Connection::Up, _) => StraightRailShape::AscendingNorth,
            (_, Connection::Up) => StraightRailShape::AscendingSouth,
            _ => StraightRailShape::NorthSouth,
        }
    } else {
        match (east, west) {
            (Connection::Up, _) => StraightRailShape::AscendingEast,
            (_, Connection::Up) => StraightRailShape::AscendingWest,
            _ => StraightRailShape::EastWest,
        }
    }
}

/// Shape for a plain rail: flat, ascending, or a curve connecting two perpendicular
/// directions.
fn rail_shape(world: &impl World, pos: BlockPos, fallback: BlockDirection) -> RailShape {
    let north = connection(world, pos, BlockDirection::North);
    let south = connection(world, pos, BlockDirection::South);
    let east = connection(world, pos, BlockDirection::East);
    let west = connection(world, pos, BlockDirection::West);

    // A curve only forms when there's a flat connection on exactly one of north/south and
    // exactly one of east/west; curves can't ascend.
    let all_flat_or_none = [north, south, east, west]
        .iter()
        .all(|c| *c != Connection::Up);
    let ns_count = (north == Connection::Flat) as u8 + (south == Connection::Flat) as u8;
    let ew_count = (east == Connection::Flat) as u8 + (west == Connection::Flat) as u8;

    if all_flat_or_none && ns_count == 1 && ew_count == 1 {
        return match (north == Connection::Flat, east == Connection::Flat) {
            (true, true) => RailShape::NorthEast,
            (true, false) => RailShape::NorthWest,
            (false, true) => RailShape::SouthEast,
            (false, false) => RailShape::SouthWest,
        };
    }

    if axis_choice(north, south, east, west, fallback) {
        match (north, south) {
            (Connection::Up, _) => RailShape::AscendingNorth,
            (_, Connection::Up) => RailShape::AscendingSouth,
            _ => RailShape::NorthSouth,
        }
    } else {
        match (east, west) {
            (Connection::Up, _) => RailShape::AscendingEast,
            (_, Connection::Up) => RailShape::AscendingWest,
            _ => RailShape::EastWest,
        }
    }
}

fn straight_shape_axis(shape: StraightRailShape) -> BlockDirection {
    use StraightRailShape::*;
    match shape {
        NorthSouth | AscendingNorth | AscendingSouth => BlockDirection::North,
        EastWest | AscendingEast | AscendingWest => BlockDirection::East,
    }
}

fn rail_shape_axis(shape: RailShape) -> BlockDirection {
    use RailShape::*;
    match shape {
        NorthSouth | AscendingNorth | AscendingSouth | NorthEast | NorthWest => {
            BlockDirection::North
        }
        EastWest | AscendingEast | AscendingWest | SouthEast | SouthWest => BlockDirection::East,
    }
}

/// The two directions a straight shape's track runs along.
pub fn shape_ends(shape: StraightRailShape) -> (BlockDirection, BlockDirection) {
    use StraightRailShape::*;
    match shape {
        NorthSouth | AscendingNorth | AscendingSouth => (BlockDirection::North, BlockDirection::South),
        EastWest | AscendingEast | AscendingWest => (BlockDirection::East, BlockDirection::West),
    }
}

pub fn rail_get_state_for_placement(world: &impl World, pos: BlockPos, facing: BlockDirection) -> Rail {
    Rail {
        shape: rail_shape(world, pos, facing),
        waterlogged: false,
    }
}

pub fn rail_on_neighbor_changed(rail: Rail, world: &impl World, pos: BlockPos) -> Rail {
    Rail {
        shape: rail_shape(world, pos, rail_shape_axis(rail.shape)),
        ..rail
    }
}

pub fn powered_rail_get_state_for_placement(
    world: &impl World,
    pos: BlockPos,
    facing: BlockDirection,
) -> PoweredRail {
    PoweredRail {
        shape: straight_shape(world, pos, facing),
        powered: false,
        waterlogged: false,
    }
}

pub fn powered_rail_on_neighbor_changed(
    rail: PoweredRail,
    world: &impl World,
    pos: BlockPos,
) -> PoweredRail {
    PoweredRail {
        shape: straight_shape(world, pos, straight_shape_axis(rail.shape)),
        ..rail
    }
}

pub fn activator_rail_get_state_for_placement(
    world: &impl World,
    pos: BlockPos,
    facing: BlockDirection,
) -> ActivatorRail {
    ActivatorRail {
        shape: straight_shape(world, pos, facing),
        powered: false,
        waterlogged: false,
    }
}

pub fn activator_rail_on_neighbor_changed(
    rail: ActivatorRail,
    world: &impl World,
    pos: BlockPos,
) -> ActivatorRail {
    ActivatorRail {
        shape: straight_shape(world, pos, straight_shape_axis(rail.shape)),
        ..rail
    }
}

pub fn detector_rail_get_state_for_placement(
    world: &impl World,
    pos: BlockPos,
    facing: BlockDirection,
) -> DetectorRail {
    DetectorRail {
        shape: straight_shape(world, pos, facing),
        powered: false,
        waterlogged: false,
    }
}

pub fn detector_rail_on_neighbor_changed(
    rail: DetectorRail,
    world: &impl World,
    pos: BlockPos,
) -> DetectorRail {
    DetectorRail {
        shape: straight_shape(world, pos, straight_shape_axis(rail.shape)),
        ..rail
    }
}

/// Walks a chain of connected same-kind rails outward from `pos` in direction `dir`, up to
/// `max` blocks, calling `visit` for each rail found (not including `pos` itself). Stops at
/// the first position that isn't a matching, axis-aligned rail (handles ascending slopes by
/// trying one block up/down when the flat neighbor doesn't connect).
pub fn walk_chain(
    world: &impl World,
    pos: BlockPos,
    dir: BlockDirection,
    kind: RailKind,
    max: u8,
    mut visit: impl FnMut(BlockPos),
) {
    let mut cursor = pos;
    for _ in 0..max {
        let flat = cursor.offset(dir.block_face());
        let mut next = None;
        for candidate in [flat, flat.offset(BlockFace::Top), flat.offset(BlockFace::Bottom)] {
            let shape = match (world.get_block(candidate), kind) {
                (Block::PoweredRail(rail), RailKind::Powered) => Some(rail.shape),
                (Block::ActivatorRail(rail), RailKind::Activator) => Some(rail.shape),
                _ => None,
            };
            if let Some(shape) = shape {
                let (a, b) = shape_ends(shape);
                if a == dir || b == dir {
                    next = Some(candidate);
                    break;
                }
            }
        }
        let Some(next_pos) = next else { break };
        visit(next_pos);
        cursor = next_pos;
    }
}

/// Whether a powered/activator rail with the given `shape` at `pos` should be powered:
/// directly, by any adjacent redstone power source, or by being within 8 rails (in either
/// direction along its own track) of another rail of the same kind that is itself directly
/// powered. Takes `shape` explicitly (rather than reading it from the world) so it can also
/// be used to compute a rail's correct initial state at placement time, before it exists in
/// the world.
pub fn is_powered(world: &impl World, pos: BlockPos, shape: StraightRailShape, kind: RailKind) -> bool {
    if super::redstone_lamp_should_be_lit(world, pos) {
        return true;
    }
    let (d1, d2) = shape_ends(shape);
    let mut powered = false;
    walk_chain(world, pos, d1, kind, 8, |p| {
        powered |= super::redstone_lamp_should_be_lit(world, p);
    });
    if !powered {
        walk_chain(world, pos, d2, kind, 8, |p| {
            powered |= super::redstone_lamp_should_be_lit(world, p);
        });
    }
    powered
}

/// After a rail's powered state changes, every other rail within its chain may need to
/// re-evaluate too (their own `is_powered` result can depend on this rail).
pub fn update_chain(world: &mut impl World, pos: BlockPos, shape: StraightRailShape, kind: RailKind) {
    let (d1, d2) = shape_ends(shape);
    for dir in [d1, d2] {
        let mut positions = Vec::new();
        walk_chain(world, pos, dir, kind, 8, |p| positions.push(p));
        for p in positions {
            super::update(world.get_block(p), world, p);
        }
    }
}
