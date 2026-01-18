mod core;
mod events;
mod heap;
mod instructions;
mod memory;
mod stack;
mod std;
mod translator;
mod types;

pub mod utils;
pub mod vm;

pub fn new(bytecode: Vec<u8>) -> vm::Vm {
    vm::Vm::new(bytecode)
}
