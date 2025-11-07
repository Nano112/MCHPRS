# Custom IO Implementation - Fixes Applied

## Problem
Custom IO wires needed to act as power sources to inject signals into redstone circuits, but wires are passive components that only relay power, not generate it.

## Solution Overview
Convert custom IO wires to `NodeType::Constant` nodes that can dynamically change power levels, while ensuring they get proper graph edges and aren't optimized away.

## Files Modified

### 1. `crates/redpiler/src/passes/identify_nodes.rs`
**Change**: Convert custom IO wires to Constants
```rust
let (ty, state) = if is_custom_io && ty == NodeType::Wire {
    (NodeType::Constant, NodeState::ss(0))  // Start at 0, set via API
} else {
    (ty, state)
};
```

### 2. `crates/redpiler/src/passes/input_search.rs`
**Changes**:
- Added `custom_io` field to `InputSearchState`
- Modified `provides_weak_power` to treat custom IO wires as power sources
- Prevented self-loop creation

### 3. `crates/redpiler/src/backend/direct/compile.rs`
**Change**: Allow Constants to have outgoing links
```rust
// Previously: Constants had no outgoing links
// Now: All nodes (including Constants) can have outgoing links
let updates: SmallVec<[ForwardLink; 10]> = graph
    .edges_directed(node_idx, Direction::Outgoing)
    // ... create update links for ALL nodes
    .collect();
```

### 4. `crates/redpiler/src/passes/analysis/ss_range_analysis.rs`
**Change**: Give custom IO Constants full signal strength range
```rust
NodeType::Constant => {
    // Custom IO Constants can have variable power
    if is_input || is_output {
        SSRange::FULL  // 0-15 range
    } else {
        SSRange::constant(state.output_strength)  // Fixed value
    }
}
```

### 5. `crates/redpiler/src/backend/direct/mod.rs`
**Change**: `set_signal_strength` already calls `set_node` which propagates to neighbors

## Key Insights

1. **Wires are passive**: `NodeType::Wire` recalculates power from inputs on every update, overwriting any manually set values

2. **Edge removal bug**: The `unreachable_output` pass was removing edges from custom IO nodes because they started at power 0, giving them `SSRange [0,0]`, causing edges with distance ≥1 to be pruned

3. **Graph vs World state**: Redpiler's internal node state must be synced to world blocks via `flush()` for changes to be visible

## Test Results
✅ `test_custom_io_injection_lights_lamp` - **PASSING**
- Proves custom IO signal injection works
- Can power lamps and redstone circuits

## Status
**Custom IO injection is now functional!** The core feature works - signals can be injected via `set_signal_strength` and will propagate through redstone circuits to power lamps and other components.

