use std::fs;
use std::fs::File;
use std::io::Write;

use crate::ast::lex;
use crate::ast::Module;
use crate::compiler::Compiler;
use crate::core::error;
use crate::core::error::ErrorType;

pub struct Run {
    args: Vec<String>,
}

impl Run {
    pub fn new(args: Vec<String>) -> Run {
        Run { args }
    }
    pub fn debug(&self) -> bool {
        self.args.contains(&"-d".to_string())
    }
    pub async fn exec(&self) {
        let module_name = if self.args.len() > 0 {
            self.args[0].clone()
        } else {
            "main.ego".to_string() // default lookup on a ego project
        };

        // run from compiled file
        if self.args.contains(&"--bytes".to_string()) {
            let bytecode = fs::read(&module_name).unwrap_or_else(|_| {
                error::throw(
                    ErrorType::FatalError,
                    format!("Cannot read {}\n", module_name).as_str(),
                    None,
                );
                std::process::exit(1); // to avoid types error
            });
            let mut vm = self_vm::new(bytecode);
            let execution = vm.run(&self.args).await;
            if let Some(err) = execution.error {
                let error_msg = format!("{}: {}", err.message, err.semantic_message);
                eprintln!("\x1b[31m[ERR] \x1b[0m{error_msg}");
            }
            return;
        }

        let file_content = fs::read_to_string(&module_name).unwrap_or_else(|_| {
            error::throw(
                ErrorType::FatalError,
                format!("Cannot read {}\n", module_name).as_str(),
                None,
            );
            std::process::exit(1); // to avoid types error
        });

        let tokens = lex(file_content);
        if self.debug() {
            println!("\nLexer tokens: \n-------------");
            for (i, token) in tokens.iter().enumerate() {
                println!("{i}. {token}");
            }
        }

        let mut module = Module::new(module_name, tokens);
        let ast = module.parse();
        if self.debug() {
            println!("\nAst nodes: \n---------------\n{:#?}", ast);
        }
        let mut compiler = Compiler::new(ast);
        let bytecode = compiler.gen_bytecode();
        let mut vm = self_vm::new(bytecode);
        let execution = vm.run(&self.args).await;
        if let Some(err) = execution.error {
            let error_msg = format!("{}: {}", err.message, err.semantic_message);
            eprintln!("\x1b[31m[ERR] \x1b[0m{error_msg}");
        }
    }
}
