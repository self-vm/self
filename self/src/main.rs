use std::fs;

fn main() {
    let path = "bytes.bin";
    let bytecode: Vec<u8> = fs::read(path).expect("cannot read bytes.bin file");

    let mut vm = self_vm::new(bytecode);
    vm.run(&vec![]);
}
