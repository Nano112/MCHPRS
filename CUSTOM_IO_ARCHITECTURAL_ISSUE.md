# Custom IO Architectural Issue

## The Core Problem

**Wires are passive** - they don't generate power, they only transmit it.

From `crates/redpiler/src/backend/direct/update.rs` lines 89-95:

```rust
NodeType::Wire => {
    let (input_power, _) = get_all_input(node);  // ← Calculates power from INPUTS
    if node.output_power != input_power {
        node.output_power = input_power;         // ← Overwrites any manually set power!
        node.changed = true;
    }
}
```

**This means:**
1. When we call `set_signal_strength` on a wire, it updates the wire's power
2. But on the next `update_node` call, the wire recalculates from its inputs
3. Since custom IO wires have no input source, they go back to power 0

## Why This Explains All Test Failures

- ✅ Monitoring natural power WORKS - we're just reading existing values
- ❌ Injection FAILS - wires reset their power to 0 based on inputs
- ❌ Relay FAILS - same reason, injected power gets overwritten

## Solution Paths

### Option 1: Convert Custom IO Wires to Constants (Current Attempt)
**Status:** Partially working but has connection issues

**Problem:** When we convert wires to Constants during `identify_nodes`, they might not get proper graph connections like wires would.

**Next Steps:**
1. Verify Constants get outgoing links during compilation
2. Ensure Constants aren't removed by optimization passes
3. Test that Constants properly power adjacent blocks

### Option 2: Skip Wire Update for Custom IO Nodes
Modify `update_node` to skip recalculation for custom IO wires:

```rust
NodeType::Wire => {
    // Skip input recalculation for custom IO nodes
    if !is_custom_io_node(node_id) {
        let (input_power, _) = get_all_input(node);
        if node.output_power != input_power {
            node.output_power = input_power;
            node.changed = true;
        }
    }
}
```

**Problem:** Requires passing custom_io info to update_node

### Option 3: New NodeType::CustomIO
Create a dedicated node type:

```rust
enum NodeType {
    // ... existing types ...
    CustomIO,  // Acts like a variable-power Constant
}
```

**Benefits:**
- Clear semantics
- Can handle both input (power generation) and output (monitoring)
- Won't be confused with other node types

**Drawbacks:**
- More invasive change
- Need to update all passes

## Recommended Approach

**Stick with NodeType::Constant conversion** but verify:

1. ✅ Constants are created correctly ← DONE
2. ✅ ss_range_analysis handles them ← FIXED  
3. ❓ Constants get proper outgoing links during compilation ← NEED TO CHECK
4. ❓ Constants aren't removed by optimization ← NEED TO CHECK
5. ❓ Constants actually power adjacent nodes ← NEED TO TEST

## Debugging Steps

### 1. Test with Real Redstone Block First

Before fixing custom IO, verify the infrastructure works by testing that a natural Constant (redstone block) can:
- Power adjacent wires
- Light lamps
- Be monitored via `get_signal_strength`

### 2. Add Debug Prints

In `identify_nodes.rs`, verify custom IO conversion:

```rust
if is_custom_io && ty == NodeType::Wire {
    eprintln!("Converting custom IO wire at {:?} to Constant", pos);
}
```

### 3. Check Link Creation

In `backend/direct/compile.rs`, verify Constant nodes get `updates` links:

```rust
let updates = if node.ty != CNodeType::Constant {
    // ... creates links ...
}
```

**This might be the bug!** Constant nodes might not be getting outgoing links!

## The Real Fix

Looking at `backend/direct/compile.rs` line 76:

```rust
let updates = if node.ty != CNodeType::Constant {
    // Create outgoing links
} else {
    vec![]  // ← Constants get NO outgoing links!
}
```

**THIS IS THE BUG!** Constants don't get outgoing update links, so they can't propagate power!

We need to:
1. Allow Constants to have outgoing links
2. OR create a new node type that acts like Constant but keeps wire-like connections
3. OR special-case custom IO Constants to get links

The fix should be in `backend/direct/compile.rs` around line 76-120.

