use std::{env, vec};

use reqwest::Client;

use crate::{
    core::error::{ai_errors::AIError, VMErrorType},
    std::ai::providers::{AIResponse, ChatRequest, ChatResponse, Message},
};

pub async fn fetch(prompt: String) -> Result<AIResponse, VMErrorType> {
    let api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY not set");

    let client = Client::new();
    let request_body = ChatRequest {
        model: "gpt-4o".to_string(),
        messages: vec![Message {
            role: "system".to_string(),
            content: prompt,
        }],
    };

    let res = client
        .post("https://api.openai.com/v1/chat/completions")
        .bearer_auth(api_key)
        .json(&request_body)
        .send()
        .await
        .expect("AI: Failed to send request");

    if !res.status().is_success() {
        return Err(VMErrorType::AI(AIError::AIFetchError(
            res.status().to_string(),
        )));
    }

    let response: ChatResponse = res.json().await.expect("AI: Failed to parse response");
    Ok(AIResponse {
        content: response.choices[0].message.content.clone(),
    })
}
