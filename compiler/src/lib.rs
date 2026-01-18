pub mod ast;
pub mod compiler;
pub mod core;
pub mod utils;

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::{env, fs};

use crate::ast::{Module, lex};
use crate::compiler::Compiler;
use crate::core::error;
use crate::core::error::ErrorType;

pub use compiler::gen_bytecode;

pub fn compile_ego(module_path: String, outname: String) {
    let file_content = fs::read_to_string(&module_path).unwrap_or_else(|_| {
        error::throw(
            ErrorType::FatalError,
            format!("Cannot read {}\n", module_path).as_str(),
            None,
        );
    });

    let tokens = lex(file_content);

    let mut module = Module::new(module_path.clone(), tokens);
    let ast = module.parse();

    let mut compiler = Compiler::new(ast);
    let bytecode = compiler.gen_bytecode();

    let input_path = Path::new(&module_path);
    let input_dir = input_path.parent().unwrap_or_else(|| {
        error::throw(
            ErrorType::FatalError,
            "Cannot determine input directory\n",
            None,
        );
    });
    let output_path: PathBuf = input_dir.join(outname);

    let mut file = match File::create(output_path) {
        Ok(file) => file,
        Err(_) => {
            error::throw(ErrorType::SyntaxError, "Cannot write file", None);
        }
    };

    match file.write_all(&bytecode) {
        Ok(_) => {}
        Err(_) => {
            error::throw(ErrorType::SyntaxError, "Cannot write file", None);
        }
    };
}
