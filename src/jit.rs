use cranelift::prelude::types::F64;
use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{DataContext, Module};

pub(crate) struct FunctionInfo {
    pub(crate) function: fn(*const f64) -> (),
    pub(crate) stack_consumed: usize,
    pub(crate) stack_returned: usize,
}

const STACK_ELEMENT_SIZE: i32 = 8;

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
    pub(crate) fn compile(&mut self, program_fragment: &str) -> FunctionInfo {
        let mut top_of_stack: Vec<Value> = vec![];
        self.ctx
            .func
            .signature
            .params
            .push(AbiParam::new(self.module.target_config().pointer_type()));

        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);
        let block = builder.create_block();

        builder.append_block_params_for_function_params(block);
        builder.switch_to_block(block);
        builder.seal_block(block);

        let mut minimum_starting_stack_size = 0;

        let mut variable_index = 0;

        // builder.block_params(block).push(AbiParam::new(self.module.target_config().pointer_type()));
        let val = builder.block_params(block)[0];
        let stack_pointer_variable = Variable::from_u32(0);
        builder.declare_var(
            stack_pointer_variable,
            self.module.target_config().pointer_type(),
        );
        builder.def_var(stack_pointer_variable, val);
        let stack_pointer = builder.use_var(stack_pointer_variable);
        let mut assert_minimum_stack_length =
            |top_of_stack: &mut Vec<Value>, builder: &mut FunctionBuilder, number: usize| {
                while top_of_stack.len() < number {
                    // let variable = Variable::from_u32(top_of_stack.len() as u32);
                    // builder.declare_var(variable, F64);
                    // top_of_stack.insert(0, builder.use_var(variable))
                    top_of_stack.insert(
                        0,
                        builder.ins().load(
                            F64,
                            MemFlags::new(),
                            stack_pointer,
                            // If this doesn't work try -2
                            (-STACK_ELEMENT_SIZE * minimum_starting_stack_size as i32),
                        ),
                    );
                    minimum_starting_stack_size += 1;
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
                '*' => {
                    assert_minimum_stack_length(&mut top_of_stack, &mut builder, 2);
                    if let (Some(a), Some(b)) = (top_of_stack.pop(), top_of_stack.pop()) {
                        top_of_stack.push(builder.ins().fmul(a, b));
                    }
                }
                '/' => {
                    assert_minimum_stack_length(&mut top_of_stack, &mut builder, 2);
                    if let (Some(a), Some(b)) = (top_of_stack.pop(), top_of_stack.pop()) {
                        top_of_stack.push(builder.ins().fdiv(a, b));
                    }
                }
                '-' => {
                    assert_minimum_stack_length(&mut top_of_stack, &mut builder, 2);
                    if let (Some(a), Some(b)) = (top_of_stack.pop(), top_of_stack.pop()) {
                        top_of_stack.push(builder.ins().fsub(a, b));
                    }
                }
                '~' => {
                    assert_minimum_stack_length(&mut top_of_stack, &mut builder, 1);
                    top_of_stack.pop();
                }
                ':' => {
                    assert_minimum_stack_length(&mut top_of_stack, &mut builder, 1);
                    if let Some(value) = top_of_stack.pop() {
                        let var = Variable::from_u32(variable_index + 1);
                        variable_index += 1;
                        builder.declare_var(var, F64);

                        builder.def_var(var, value);
                        top_of_stack.push(builder.use_var(var));
                        top_of_stack.push(builder.use_var(var));
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

        let final_stack_length = top_of_stack.len();
        for (index, element) in top_of_stack.into_iter().enumerate() {
            let var = builder.use_var(stack_pointer_variable);

            builder.ins().store(
                MemFlags::new(),
                element,
                var,
                -STACK_ELEMENT_SIZE * (minimum_starting_stack_size as i32 - index as i32 - 1),
            );
        }

        builder.ins().return_(&[]);

        builder.finalize();

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

        return FunctionInfo {
            function: unsafe { std::mem::transmute::<_, fn(*const f64) -> ()>(code) },
            stack_returned: final_stack_length,
            stack_consumed: minimum_starting_stack_size,
        };
    }
}
