use crate::std::http::utils::sanitize_html_for_llm;

// THIS FUNCTION SANITIZES THE GIVEN INPUT BUFFER FOR
// ITS INTRODUCTION TO AN LLM
pub fn sanitize(value_bytes: Vec<u8>, content_type: &str) -> String {
    match content_type {
        "text/html" => sanitize_html_for_llm(&String::from_utf8_lossy(&value_bytes).to_string()),
        _ => String::from_utf8_lossy(&value_bytes).to_string(),
    }
}

pub fn is_sanitizable(content: &str) -> Option<&str> {
    let content_lower = content.trim_start().to_lowercase();

    let has_html_tags = content_lower.contains("<html")
        || content_lower.contains("<body")
        || content_lower.contains("<div")
        || content_lower.contains("<p");

    let is_doctype = content_lower.starts_with("<!doctype html");

    if is_doctype || has_html_tags {
        Some("text/html")
    } else {
        None
    }
}
