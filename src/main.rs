mod jit;

fn main() {
    let program = "123@++";

    let mut compiler = jit::Jit::default();
    let program = compiler.compile(program);

    let function = unsafe { std::mem::transmute::<_, extern "C" fn() -> [f64; 1]>(program) };

    let result = function();
    println!("{:?}", result);
}
