pub mod ast;
pub mod compiler;
pub mod core;
pub mod utils;

use std::path::PathBuf;
use std::fs;

use crate::ast::{Module, lex};
use crate::compiler::Compiler;
use crate::core::error;
use crate::core::error::ErrorType;

pub use compiler::gen_bytecode;

pub fn compile_ego(module_path: PathBuf) -> Vec<u8> {
    let file_content = fs::read_to_string(&module_path).unwrap_or_else(|_| {
        error::throw(
            ErrorType::FatalError,
            format!("Cannot read {}\n", module_path.to_string_lossy()).as_str(),
            None,
        );
    });

    let tokens = lex(file_content);

    let mut module = Module::new(module_path.to_string_lossy().to_string(), tokens);
    let ast = module.parse();

    let mut compiler = Compiler::new(ast);
    let bytecode = compiler.gen_bytecode();

    bytecode
}

pub fn unsafe_compile_block(module_name: &str, module_path: PathBuf) -> Vec<u8> {
    let file_content = fs::read_to_string(&module_path).unwrap_or_else(|_| {
        error::throw(
            ErrorType::FatalError,
            format!("Cannot read {}\n", module_path.to_string_lossy()).as_str(),
            None,
        );
    });

    let tokens = lex(file_content);
    let mut module = Module::new(module_name.to_string(), tokens);
    let block_node = module.parse_block();

    let bytecode = Compiler::compile_block(&block_node);
    bytecode
}

pub fn unsafe_compile_block_from_str(module_name: &str, file_content: String) -> Vec<u8> {
    let tokens = lex(file_content);
    let mut module = Module::new(module_name.to_string(), tokens);
    let block_node = module.parse_block();

    let bytecode = Compiler::compile_block(&block_node);

    bytecode
}
