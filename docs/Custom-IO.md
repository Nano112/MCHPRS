# Custom IO Nodes

## Overview

The Custom IO feature extends Redpiler's capabilities by allowing arbitrary redstone components to be designated as input/output nodes. This enables precise control and monitoring of signal strengths at any position in a redstone circuit, making it ideal for debugging, testing, and advanced simulation scenarios.

## What It Does

- **Signal Injection**: Set signal strengths at any redstone component position
- **Signal Monitoring**: Read signal strengths from any component in real-time
- **Debugging Support**: Create probe points throughout circuits for analysis
- **Simulation Control**: Build interactive redstone simulators with arbitrary IO points

## Why It's Useful

### For Circuit Debugging
```rust
// Monitor signal propagation through complex circuits
let probe_positions = vec![
    BlockPos::new(10, 5, 10), // Input dust
    BlockPos::new(15, 5, 10), // Internal repeater
    BlockPos::new(20, 5, 10), // Output lamp
];

world.set_signal_strength(BlockPos::new(10, 5, 10), 12);
world.tick();
let output = world.get_signal_strength(BlockPos::new(20, 5, 10));
```

### For Automated Testing
- Test circuit behavior under various input conditions
- Verify signal propagation timing
- Create comprehensive test suites for redstone contraptions

### For Educational Tools
- Build interactive redstone circuit simulators
- Visualize signal flow in real-time
- Create debugging tools for redstone engineers

### For Advanced Applications
- Generate heatmaps of redstone activity
- Implement custom redstone logic analyzers
- Create automated redstone testing frameworks

## How It Works

### Configuration
```rust
let options = SimulationOptions {
    custom_io: vec![
        BlockPos::new(x, y, z), // Any redstone component position
    ],
    ..Default::default()
};
```

### Usage
```rust
// Inject signal
world.set_signal_strength(BlockPos::new(x, y, z), strength);

// Monitor signal
let current_strength = world.get_signal_strength(BlockPos::new(x, y, z));
```

## Technical Details

- **No Performance Impact**: Custom IO nodes are preserved during optimization
- **Full Compatibility**: Works with all existing Redpiler features
- **Real-time Access**: Read/write signals during active simulation
- **Type Agnostic**: Works with dust, repeaters, comparators, and other components

## Integration

The feature integrates seamlessly with existing Redpiler functionality:
- Compatible with `io_only` mode for faster simulation
- Preserves all optimization passes
- Maintains backward compatibility
- Follows established API patterns

## Use Cases

- **Circuit Analysis**: Probe internal nodes in complex mechanisms
- **Automated Testing**: Create test harnesses for redstone builds
- **Educational Software**: Build interactive learning tools
- **Debugging Tools**: Develop advanced redstone analysis software
- **Simulation Engines**: Create custom redstone simulation environments</content>
<parameter name="filePath">/Users/harrison/Documents/GitHub/MCHPRS/docs/Custom-IO.md