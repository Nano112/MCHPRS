# Custom IO Debug Suggestions

## Current Issue

Tests are failing even with the `schedule_tick` fix. The signal is stored but not powering the circuit.

## Diagnostic: Check Node Type

First, let's verify what type of node a custom IO wire is:

```rust
fn set_signal_strength(&mut self, pos: BlockPos, strength: u8) {
    if let Some(&node_id) = self.pos_map.get(&pos) {
        let node = &self.nodes[node_id];
        eprintln!("=== Custom IO Debug ===");
        eprintln!("Position: {:?}", pos);
        eprintln!("Node ID: {}", node_id);
        eprintln!("Node Type: {:?}", node.ty);
        eprintln!("Old Power: {}", node.output_power);
        eprintln!("New Power: {}", strength);
        eprintln!("Num updates: {}", node.updates.len());
        
        // Current implementation
        self.schedule_tick(node_id, 0, TickPriority::Highest);
        self.set_node(node_id, strength > 0, strength);
    } else {
        warn!("Tried to set signal strength at position {} which is not a redpiler node", pos);
    }
}
```

## Alternative Fix 1: Don't Schedule, Just Set

Maybe scheduling interferes. Try:

```rust
fn set_signal_strength(&mut self, pos: BlockPos, strength: u8) {
    if let Some(&node_id) = self.pos_map.get(&pos) {
        // Just set the node, let set_node handle propagation
        self.set_node(node_id, strength > 0, strength);
    } else {
        warn!("Tried to set signal strength at position {} which is not a redpiler node", pos);
    }
}
```

## Alternative Fix 2: Set THEN Schedule

```rust
fn set_signal_strength(&mut self, pos: BlockPos, strength: u8) {
    if let Some(&node_id) = self.pos_map.get(&pos) {
        // Set first, then schedule
        self.set_node(node_id, strength > 0, strength);
        self.schedule_tick(node_id, 0, TickPriority::Highest);
    } else {
        warn!("Tried to set signal strength at position {} which is not a redpiler node", pos);
    }
}
```

## Alternative Fix 3: Force Immediate Update

```rust
fn set_signal_strength(&mut self, pos: BlockPos, strength: u8) {
    if let Some(&node_id) = self.pos_map.get(&pos) {
        self.set_node(node_id, strength > 0, strength);
        
        // Force immediate update (might need to import update module)
        update::update_node(
            &mut self.scheduler,
            &mut self.events,
            &mut self.nodes,
            node_id,  // Update the node itself, not just neighbors
        );
    } else {
        warn!("Tried to set signal strength at position {} which is not a redpiler node", pos);
    }
}
```

## Alternative Fix 4: Mark as Changed

Looking at `set_node`, it marks `node.changed = true`. Maybe we need to ensure this propagates:

```rust
fn set_signal_strength(&mut self, pos: BlockPos, strength: u8) {
    if let Some(&node_id) = self.pos_map.get(&pos) {
        self.set_node(node_id, strength > 0, strength);
        
        // Ensure the node is marked for update
        let node = &mut self.nodes[node_id];
        node.changed = true;
        node.pending_tick = false;  // Clear any pending ticks that might interfere
    } else {
        warn!("Tried to set signal strength at position {} which is not a redpiler node", pos);
    }
}
```

## Hypothesis

The issue might be that:

1. **Wire nodes behave differently than levers** - Levers are input nodes that generate power. Wires are passive and only transmit power from inputs.

2. **Custom IO wires might not be marked correctly** - Check in `identify_nodes.rs` if custom IO positions are being marked as BOTH input AND output correctly.

3. **The node might need to act as a power source** - Maybe custom IO nodes need to be converted to `NodeType::Constant` with the specified power level?

## Recommended Debugging Steps

1. Add debug prints to see what node type custom IO positions become
2. Check if `node.updates` is non-empty (are there neighbors to propagate to?)
3. Try Alternative Fix 1 first (simplest - just remove the schedule_tick)
4. If that doesn't work, try Alternative Fix 3 (force update on the node itself)

## Check identify_nodes.rs

Verify in `crates/redpiler/src/passes/identify_nodes.rs` that custom IO nodes are being correctly identified:

```rust
let is_custom_io = custom_io.contains(&pos);
let is_input = matches!(
    ty,
    NodeType::Button | NodeType::Lever | NodeType::PressurePlate
) || is_custom_io;  // ← Should make custom IO act like an input

let is_output = matches!(
    ty,
    NodeType::Trapdoor | NodeType::Lamp | NodeType::NoteBlock { .. }
) || matches!(block, Block::RedstoneWire { wire } if wire_dot_out && wire::is_dot(wire))
|| is_custom_io;  // ← Should also be an output
```

This looks correct, but maybe wires need special handling to act as power sources?

