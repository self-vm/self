mod ast;
mod commands;
mod compiler;
mod core;

use commands::Command;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let command = Command::parse();
    command.exec().await;
}
