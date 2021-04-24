use super::JITBackend;
use crate::blocks::{self, Block, BlockPos};
use crate::redpiler::{Node, Link, LinkType};
use crate::world::{TickEntry, TickPriority};
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{DataContext, Linkage, Module, DataId};
use log::{warn, debug};
use std::collections::HashMap;

struct CLTickEntry {
    ticks_left: u32,
    priority: TickPriority,
    tick_fn: extern "C" fn(&mut CraneliftBackend),
}

struct FunctionTranslator<'a> {
    builder: FunctionBuilder<'a>,
    module: &'a mut JITModule,
    output_power_data: &'a [DataId],
    comparator_output_data: &'a [DataId],
    node_idx: usize, 
    node: &'a Node, 
    nodes: &'a [Node]
}

impl<'a> FunctionTranslator<'a> {
    fn translate_output_power(&mut self, idx: usize) -> Value {
        let gv = if matches!(self.node.state, Block::RedstoneComparator { .. }) || self.node.state.has_comparator_override() {
            self.module.declare_data_in_func(self.comparator_output_data[idx], &mut self.builder.func)
        } else {
            self.module.declare_data_in_func(self.output_power_data[idx], &mut self.builder.func)
        };
        self.builder.ins().symbol_value(types::I8, gv)
    }

    /// Recursive method that returns (input_power, side_input_power)
    fn translate_node_input_power_recur(&mut self, inputs: &[Link], input_power: Value, side_input_power: Value) -> (Value, Value) {
        debug!("xd");
        match inputs.first() {
            Some(input) => match input.ty {
                LinkType::Default => {
                    let v = self.translate_output_power(input.end.index);
                    let new_input_power = self.builder.ins().iadd(v, input_power);
                    self.translate_node_input_power_recur(&inputs[1..], new_input_power, side_input_power)
                }
                LinkType::Side => {
                    let v = self.translate_output_power(input.end.index);
                    let new_side_input_power = self.builder.ins().iadd(v, side_input_power);
                    self.translate_node_input_power_recur(&inputs[1..], input_power, new_side_input_power)
                }
            }
            None => (input_power, side_input_power)
        }
    }

    /// returns (input_power, side_input_power)
    fn translate_node_input_power(&mut self, inputs: &[Link]) -> (Value, Value) {
        let input_power = self.builder.ins().iconst(types::I8, 0);
        let side_input_power = self.builder.ins().iconst(types::I8, 0);
        self.translate_node_input_power_recur(inputs, input_power, side_input_power)
    }

    fn translate_update(&mut self) {
        match self.node.state {
            Block::RedstoneComparator { .. } => self.translate_comparator_update(),
            Block::RedstoneTorch { .. } => {}
            Block::RedstoneWallTorch { .. } => {}
            Block::RedstoneRepeater { .. } => {}
            Block::RedstoneWire { .. } => {}
            Block::Lever { .. } => {}
            Block::StoneButton { .. } => {}
            Block::RedstoneBlock { .. } => {}
            Block::RedstoneLamp { .. } => {}
            state => warn!("Trying to compile node with state {:?}", state),
        }
        self.builder.ins().return_(&[]);
    }

    fn translate_tick(&mut self) {
        match self.node.state {
            Block::RedstoneComparator { .. } => self.translate_comparator_tick(),
            Block::RedstoneTorch { .. } => {}
            Block::RedstoneWallTorch { .. } => {}
            Block::RedstoneRepeater { .. } => {}
            Block::RedstoneWire { .. } => {}
            Block::Lever { .. } => {}
            Block::StoneButton { .. } => {}
            Block::RedstoneBlock { .. } => {}
            Block::RedstoneLamp { .. } => {}
            state => warn!("Trying to compile node with state {:?}", state),
        }
        self.builder.ins().return_(&[]);
    }

    fn translate_comparator_update(&mut self) {
        let (input, side_input) = self.translate_node_input_power(&self.node.inputs);
    }

    fn translate_comparator_tick(&mut self) {
        let (input, side_input) = self.translate_node_input_power(&self.node.inputs);
    }
}

pub struct CraneliftBackend {
    // Compilation
    builder_context: FunctionBuilderContext,
    ctx: codegen::Context,
    module: JITModule,
    // Execution
    initial_nodes: Vec<Node>,
    tick_fns: Vec<extern "C" fn(&mut CraneliftBackend)>,
    use_fns: Vec<extern "C" fn(&mut CraneliftBackend)>,
    pos_map: HashMap<BlockPos, usize>,
    to_be_ticked: Vec<CLTickEntry>,
    change_queue: Vec<(BlockPos, Block)>,
}

impl Default for CraneliftBackend {
    fn default() -> Self {
        let builder = JITBuilder::new(cranelift_module::default_libcall_names());
        let module = JITModule::new(builder);
        Self {
            builder_context: FunctionBuilderContext::new(),
            ctx: module.make_context(),
            module,
            initial_nodes: Default::default(),
            tick_fns: Default::default(),
            use_fns: Default::default(),
            pos_map: Default::default(),
            to_be_ticked: Default::default(),
            change_queue: Default::default(),
        }
    }
}

impl JITBackend for CraneliftBackend {
    fn compile(&mut self, nodes: Vec<Node>, ticks: Vec<TickEntry>) {
        let mut data_ctx = DataContext::new();

        let mut output_power_data = Vec::new();
        let mut comparator_output_data = Vec::new();
        for idx in 0..nodes.len() {
            let output_power_name = format!("n{}_output_power", idx);
            let comparator_output_name = format!("n{}_comparator_output", idx);

            data_ctx.define_zeroinit(1);
            let output_power_id = self
                .module
                .declare_data(&output_power_name, Linkage::Local, true, false)
                .unwrap();
            output_power_data.push(output_power_id);
            self.module.define_data(output_power_id, &data_ctx).unwrap();
            data_ctx.clear();

            data_ctx.define_zeroinit(1);
            let comparator_output_id = self
                .module
                .declare_data(&comparator_output_name, Linkage::Local, true, false)
                .unwrap();
            comparator_output_data.push(comparator_output_id);
            self.module
                .define_data(comparator_output_id, &data_ctx).unwrap();
            data_ctx.clear();
        }

        for (idx, node) in nodes.iter().enumerate() {
            let mut update_builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);
            let update_entry_block = update_builder.create_block();
            update_builder.switch_to_block(update_entry_block);
            update_builder.seal_block(update_entry_block);

            let mut update_translator = FunctionTranslator {
                builder: update_builder,
                module: &mut self.module,
                comparator_output_data: &comparator_output_data,
                output_power_data: &output_power_data,
                node,
                node_idx: idx,
                nodes: &nodes
            };
            update_translator.translate_update();
            debug!("n{}_update generated {}", idx, &update_translator.builder.func);

            update_translator.builder.finalize();
            let update_id = self
                .module
                .declare_function(&format!("n{}_update", idx), Linkage::Export, &self.ctx.func.signature)
                .unwrap();
            self.module
                .define_function(update_id, &mut self.ctx, &mut codegen::binemit::NullTrapSink {}, &mut codegen::binemit::NullStackMapSink {})
                .unwrap();
            self.module.clear_context(&mut self.ctx);

            let mut tick_builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);
            let tick_entry_block = tick_builder.create_block();
            tick_builder.switch_to_block(tick_entry_block);
            tick_builder.seal_block(tick_entry_block);

            let mut tick_translator = FunctionTranslator {
                builder: tick_builder,
                module: &mut self.module,
                comparator_output_data: &comparator_output_data,
                output_power_data: &output_power_data,
                node,
                node_idx: idx,
                nodes: &nodes
            };
            tick_translator.translate_tick();
            debug!("n{}_tick generated {}", idx, &tick_translator.builder.func);

            tick_translator.builder.finalize();
            let tick_id = self
                .module
                .declare_function(&format!("n{}_tick", idx), Linkage::Export, &self.ctx.func.signature)
               .unwrap();
            self.module
                .define_function(tick_id, &mut self.ctx, &mut codegen::binemit::NullTrapSink {}, &mut codegen::binemit::NullStackMapSink {})
                .unwrap();
            self.module.clear_context(&mut self.ctx);
        }

        self.module.finalize_definitions();

        for (i, node) in nodes.iter().enumerate() {
            self.pos_map.insert(node.pos, i);
        }

        for entry in ticks {
            self.to_be_ticked.push(CLTickEntry {
                ticks_left: entry.ticks_left,
                priority: entry.tick_priority,
                tick_fn: self.tick_fns[self.pos_map[&entry.pos]],
            })
        }

        self.initial_nodes = nodes;
    }

    fn tick(&mut self) {
        self.to_be_ticked
            .sort_by_key(|e| (e.ticks_left, e.priority));
        while self.to_be_ticked.first().map(|e| e.ticks_left).unwrap_or(1) == 0 {
            let entry = self.to_be_ticked.remove(0);
            (entry.tick_fn)(self);
        }
    }

    fn on_use_block(&mut self, pos: BlockPos) {
        self.use_fns[self.pos_map[&pos]](self);
    }

    fn reset(&mut self) -> Vec<TickEntry> {
        let mut ticks = Vec::new();
        for entry in self.to_be_ticked.drain(..) {
            ticks.push(TickEntry {
                ticks_left: entry.ticks_left,
                tick_priority: entry.priority,
                pos: self.initial_nodes[self
                    .tick_fns
                    .iter()
                    .position(|f| *f as usize == entry.tick_fn as usize)
                    .unwrap()]
                .pos,
            })
        }
        ticks
    }

    fn block_changes(&mut self) -> &mut Vec<(BlockPos, blocks::Block)> {
        &mut self.change_queue
    }
}

#[no_mangle]
extern "C" fn cranelift_jit_schedule_tick(
    backend: &mut CraneliftBackend,
    delay: u32,
    priority: u8,
    tick_fn: extern "C" fn(&mut CraneliftBackend),
) {
    backend.to_be_ticked.push(CLTickEntry {
        ticks_left: delay,
        priority: match priority {
            0 => TickPriority::Normal,
            1 => TickPriority::High,
            2 => TickPriority::Higher,
            3 => TickPriority::Highest,
            _ => panic!("Cranelift JIT scheduled tick with priority of {}", priority),
        },
        tick_fn,
    })
}
