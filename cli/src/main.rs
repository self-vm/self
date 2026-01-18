#![allow(warnings)]

mod commands;
use commands::Command;

#[tokio::main(flavor = "multi_thread")]
async fn main() {
    let command = Command::parse();
    command.exec().await;
}
