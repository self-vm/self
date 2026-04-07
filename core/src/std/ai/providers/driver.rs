use std::env;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;

use crate::{
    core::error::{ai_errors::AIError, VMErrorType},
    std::ai::providers::AIResponse,
};

pub async fn fetch(prompt: String) -> Result<AIResponse, VMErrorType> {
    let socket_path = env::var("DRIVER_SOCKET").unwrap_or_else(|_| "driver.sock".to_string());

    let mut stream = UnixStream::connect(&socket_path).await.map_err(|e| {
        VMErrorType::AI(AIError::AIFetchError(format!(
            "DRIVER: failed to connect to socket: {}",
            e
        )))
    })?;

    stream.write_all(prompt.as_bytes()).await.map_err(|e| {
        VMErrorType::AI(AIError::AIFetchError(format!(
            "DRIVER: failed to write prompt: {}",
            e
        )))
    })?;

    stream.shutdown().await.map_err(|e| {
        VMErrorType::AI(AIError::AIFetchError(format!(
            "DRIVER: failed to shutdown write side: {}",
            e
        )))
    })?;

    let mut raw = String::new();
    stream.read_to_string(&mut raw).await.map_err(|e| {
        VMErrorType::AI(AIError::AIFetchError(format!(
            "DRIVER: failed to read response: {}",
            e
        )))
    })?;

    let content = raw
        .replace("<DRIVER_START>", "")
        .replace("<DRIVER_END>", "")
        .trim()
        .to_string();

    Ok(AIResponse { content })
}
