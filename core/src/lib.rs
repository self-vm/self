pub mod core;
pub mod events;
pub mod heap;
pub mod instructions;
pub mod memory;
pub mod stack;
pub mod std;
pub mod translator;
pub mod types;

pub mod utils;
pub mod vm;

pub fn new(bytecode: Vec<u8>) -> vm::Vm {
    vm::Vm::new(bytecode)
}
