# Custom IO Adjacent Block Bug

## Summary

Custom IO redstone wire that is **directly adjacent to a solid block** does not correctly power that block or components on top of it.

## Bug Description

When a custom IO redstone wire is placed directly next to a solid block (like concrete), setting the wire's signal strength does not properly power the adjacent block. This prevents components on top of the block (like redstone torches) from responding to the signal.

## Reproduction

### Failing Case (Bug)

```
Circuit layout (side view):
Y=2: ^ (redstone torch on top of concrete)
Y=1: c (concrete block)
Y=0: │ (custom IO redstone wire - directly adjacent)
```

**Steps:**
1. Place redstone wire at (0, 0, 0) and mark it as custom IO
2. Place concrete block at (0, 1, 0)
3. Place redstone torch at (0, 2, 0)
4. Compile the circuit with redpiler
5. Set custom IO wire signal strength to 15
6. Tick the simulation

**Expected:** Torch turns OFF (wire powers concrete, concrete turns off torch)
**Actual:** Torch stays ON (signal does not propagate)

### Working Case (Workaround)

```
Circuit layout (side view):
Y=3: ^ (redstone torch on top of concrete)
Y=2: c (concrete block)
Y=1: │ (regular redstone wire - NOT custom IO)
Y=0: │ (custom IO redstone wire - with gap)
```

**Steps:**
1. Place custom IO wire at (0, 0, 0)
2. Place regular wire at (0, 1, 0)
3. Place concrete at (0, 2, 0)
4. Place torch at (0, 3, 0)
5. Compile and run

**Result:** Signal propagates correctly through the gap, torch turns OFF as expected.

## Root Cause (Hypothesis)

The issue likely occurs in one of these areas:

1. **`identify_nodes` pass** (`crates/redpiler/src/passes/identify_nodes.rs`)
   - Custom IO wires may not be properly identified as signal sources when adjacent to blocks
   - The pass might skip registering the wire-to-block power relationship

2. **`input_search` pass** (`crates/redpiler/src/passes/input_search.rs`)
   - Custom IO inputs might not be searched for adjacent powered blocks
   - The search algorithm may terminate early for custom IO nodes

3. **Backend update logic** (`crates/redpiler/src/backend/direct/update.rs`)
   - When custom IO signal strength changes, adjacent block updates might not be triggered
   - The update propagation may not include blocks directly adjacent to custom IO

## Test Code

To reproduce this bug programmatically, create a test in `crates/redpiler/tests/`:

```rust
// This test should FAIL until the bug is fixed
#[test]
fn test_custom_io_powers_adjacent_block() {
    // Setup: Create world with wire->concrete->torch stack
    // Mark wire as custom IO
    // Set wire power to 15
    // Tick simulation
    // Assert: torch.lit == false (currently fails, torch stays lit)
}
```

## Impact

This bug affects any circuit that uses custom IO positioned directly adjacent to solid blocks, including:
- D-latches with custom IO clock/data inputs
- Memory cells
- Any compact circuit design where space is limited

## Workaround

Add a gap (one regular redstone wire) between the custom IO wire and any solid blocks that need to be powered by it.

## Files to Investigate

1. `crates/redpiler/src/passes/identify_nodes.rs` - Node identification for custom IO
2. `crates/redpiler/src/passes/input_search.rs` - Input searching and power propagation
3. `crates/redpiler/src/backend/direct/update.rs` - Block update logic
4. `crates/redpiler/src/backend/direct/compile.rs` - Compilation of custom IO nodes
5. `crates/redstone/src/wire/mod.rs` - Redstone wire power calculation

## Related Code Patterns

Look for code that:
- Checks `if is_custom_io(pos)` and handles it differently
- Iterates over adjacent blocks to propagate power
- Updates block states when input signals change
- Converts custom IO wires to constant nodes in the graph


