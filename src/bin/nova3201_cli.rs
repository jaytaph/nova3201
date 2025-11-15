use nova3201::Machine;

fn main() {
    let mut mach = Machine::new();

    println!("Loading program...");
    let program: [u32; 0] = [];

    mach.load_program(0x0001_0000, &program);

    for _ in 0..10_000 {
        mach.step();
        if mach.cpu.halted {
            println!("CPU halted.");
            break;
        }
    }
}
