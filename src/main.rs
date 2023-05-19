mod jit;

fn main() {
    let program = "312*32*+::**:$~";

    let mut stack = vec![0.];

    let mut compiler = jit::Jit::default();
    let program = compiler.compile(program);
    let original_stack_length = stack.len();
    stack.resize(
        stack
            .len()
            .max(stack.len() - program.stack_consumed + program.stack_returned),
        0.0,
    );
    (program.function)(&stack[original_stack_length - 1]);
    stack.resize(
        original_stack_length - program.stack_consumed + program.stack_returned,
        0.0,
    );
    println!("{:?}", stack);
}
