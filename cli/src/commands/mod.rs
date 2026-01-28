pub mod compile;
pub mod logo;
pub mod new;
pub mod run;
pub mod test;
pub mod chat;

use ego_compiler::core::error::{self, ErrorType};

use self::logo::Logo;
use self::new::New;
use self::run::Run;
use self::test::Test;
use self::chat::Chat;

use crate::commands::compile::Compile;
use std::env;

pub enum Command {
    Run(Run),
    Logo(Logo),
    New(New),
    Compile(Compile),
    Test(Test),
    Chat(Chat),
    Raw,
}

impl Command {
    pub fn parse() -> Command {
        let args: Vec<String> = env::args().collect();
        if args.len() >= 2 {
            let command = args[1].clone();
            let remaining_args = &args[2..];
            return Command::cmd_from_str(command.as_str(), remaining_args.to_vec());
        } else {
            return Command::Chat(Chat::new(vec![]));
        };
    }
    fn cmd_from_str(command: &str, args: Vec<String>) -> Command {
        match command {
            "run" => Command::Run(Run::new(args)),
            "logo" => Command::Logo(Logo::new(args)),
            "new" => Command::New(New::new(args)),
            "compile" => Command::Compile(Compile::new(args)),
            "test" => Command::Test(Test::new(args)),
            "chat" => Command::Chat(Chat::new(args)),
            "ping" => Command::Raw,
            _ => {
                // Check if it's a file, if so run it, otherwise treat as a natural language prompt
                if !command.is_empty() && (command.ends_with(".ego") || std::path::Path::new(command).exists()) {
                    Command::Run(Run::new(
                        [command.to_string()]
                            .into_iter()
                            .chain(args.into_iter())
                            .collect(),
                    ))
                } else {
                    Command::Chat(Chat::new(
                        [command.to_string()]
                            .into_iter()
                            .chain(args.into_iter())
                            .collect(),
                    ))
                }
            }
        }
    }
    pub async fn exec(&self) {
        match self {
            Command::Run(v) => v.exec().await,
            Command::Logo(v) => v.exec(),
            Command::New(v) => v.exec(),
            Command::Compile(v) => v.exec(),
            Command::Test(v) => v.exec().await,
            Command::Chat(v) => v.exec().await,
            Command::Raw => {
                println!("pong!")
            }
        }
    }
}
