use std::{env, fs, process};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("usage: {} <bytecode_file>", args[0]);
        std::process::exit(1);
    }
    let path = args[1].clone();
    let bytecode: Vec<u8> = if let Ok(bytes) = fs::read(path) {
        bytes
    } else {
        eprintln!("\ncannot read bytecode file: {}\n", args[1]);
        process::exit(1);
    };

    let mut vm = self_vm::new(bytecode);
    vm.run(&vec![]);
}
