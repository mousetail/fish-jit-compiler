use cranelift::prelude::types::F64;
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{DataContext, Linkage, Module};
use std::collections::HashMap;
use std::slice;

pub(crate) struct Jit {
    /// The function builder context, which is reused across multiple
    /// FunctionBuilder instances.
    builder_context: FunctionBuilderContext,

    /// The main Cranelift context, which holds the state for codegen. Cranelift
    /// separates this from `Module` to allow for parallel compilation, with a
    /// context per thread, though this isn't in the simple demo here.
    ctx: codegen::Context,

    /// The data context, which is to data objects what `ctx` is to functions.
    data_ctx: DataContext,

    /// The module, with the jit backend, which manages the JIT'd
    /// functions.
    module: JITModule,
}

impl Default for Jit {
    fn default() -> Self {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "false").unwrap();
        let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
            panic!("host machine is not supported: {}", msg);
        });
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .unwrap();
        let builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());

        let module = JITModule::new(builder);
        Self {
            builder_context: FunctionBuilderContext::new(),
            ctx: module.make_context(),
            data_ctx: DataContext::new(),
            module,
        }
    }
}

impl Jit {
    pub(crate) fn compile(&mut self, program_fragment: &str) -> *const u8 {
        let mut top_of_stack: Vec<Value> = vec![];

        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);
        let block = builder.create_block();
        builder.append_block_params_for_function_params(block);
        builder.switch_to_block(block);
        builder.seal_block(block);

        let mut minimum_starting_stack_size = 0;

        let mut assert_minimum_stack_length =
            |top_of_stack: &mut Vec<Value>, builder: &mut FunctionBuilder, number: usize| {
                while top_of_stack.len() < number {
                    minimum_starting_stack_size += 1;
                    let variable = Variable::from_u32(top_of_stack.len() as u32);
                    builder.declare_var(variable, F64);
                    top_of_stack.insert(0, builder.use_var(variable))
                }
            };

        for instruction in program_fragment.chars() {
            match instruction {
                '$' => {
                    assert_minimum_stack_length(&mut top_of_stack, &mut builder, 2);
                    let length = top_of_stack.len();
                    top_of_stack[length - 2..].rotate_left(1);
                }
                '@' => {
                    assert_minimum_stack_length(&mut top_of_stack, &mut builder, 3);
                    let length = top_of_stack.len();
                    top_of_stack[length - 3..].rotate_left(1);
                }
                '+' => {
                    assert_minimum_stack_length(&mut top_of_stack, &mut builder, 2);
                    if let (Some(a), Some(b)) = (top_of_stack.pop(), top_of_stack.pop()) {
                        top_of_stack.push(builder.ins().fadd(a, b));
                    }
                }
                '0' => top_of_stack.push(builder.ins().f64const(0.)),
                '1' => top_of_stack.push(builder.ins().f64const(1.)),
                '2' => top_of_stack.push(builder.ins().f64const(2.)),
                '3' => top_of_stack.push(builder.ins().f64const(3.)),
                '4' => top_of_stack.push(builder.ins().f64const(4.)),
                '5' => top_of_stack.push(builder.ins().f64const(5.)),
                '6' => top_of_stack.push(builder.ins().f64const(6.)),
                '7' => top_of_stack.push(builder.ins().f64const(7.)),
                '8' => top_of_stack.push(builder.ins().f64const(8.)),
                '9' => top_of_stack.push(builder.ins().f64const(9.)),
                'a' => top_of_stack.push(builder.ins().f64const(10.)),
                'b' => top_of_stack.push(builder.ins().f64const(11.)),
                'c' => top_of_stack.push(builder.ins().f64const(12.)),
                'd' => top_of_stack.push(builder.ins().f64const(13.)),
                'e' => top_of_stack.push(builder.ins().f64const(14.)),
                'f' => top_of_stack.push(builder.ins().f64const(15.)),
                _ => (),
            };
        }
        drop(assert_minimum_stack_length);

        builder.ins().return_(&top_of_stack);

        builder.finalize();

        for _ in 0..minimum_starting_stack_size {
            self.ctx.func.signature.params.push(AbiParam::new(F64));
        }
        for _ in 0..top_of_stack.len() {
            self.ctx.func.signature.returns.push(AbiParam::new(F64));
        }

        let function = self
            .module
            .declare_anonymous_function(&self.ctx.func.signature)
            .unwrap();

        self.module
            .define_function(function, &mut self.ctx)
            .unwrap();

        self.module.clear_context(&mut self.ctx);

        self.module.finalize_definitions().unwrap();

        let code = self.module.get_finalized_function(function);

        return code;
    }
}
