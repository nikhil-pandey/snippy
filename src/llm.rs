use serde::{Deserialize, Serialize};
use crate::errors::ClipboardError;
use reqwest::Client;
use std::env;
use tracing::info;
use std::ops::Add;

#[derive(Debug, Deserialize, Default, Clone, Copy)]
pub struct CompletionTokenDetails {
    pub reasoning_tokens: u32,
    pub accepted_prediction_tokens: u32,
    pub rejected_prediction_tokens: u32,
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    #[serde(default)]
    pub completion_tokens_details: Option<CompletionTokenDetails>,
}

impl Add for TokenUsage {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            prompt_tokens: self.prompt_tokens + other.prompt_tokens,
            completion_tokens: self.completion_tokens + other.completion_tokens,
            total_tokens: self.total_tokens + other.total_tokens,
            completion_tokens_details: match (self.completion_tokens_details, other.completion_tokens_details) {
                (Some(a), Some(b)) => Some(CompletionTokenDetails {
                    reasoning_tokens: a.reasoning_tokens + b.reasoning_tokens,
                    accepted_prediction_tokens: a.accepted_prediction_tokens + b.accepted_prediction_tokens,
                    rejected_prediction_tokens: a.rejected_prediction_tokens + b.rejected_prediction_tokens,
                }),
                _ => None,
            },
        }
    }
}

#[derive(Debug)]
pub struct LLMResponse {
    pub content: String,
    pub usage: TokenUsage,
}

pub struct LLMClient {
    client: Client,
    model: String,
}

impl LLMClient {
    pub fn new(model: String) -> Self {
        LLMClient {
            client: Client::new(),
            model,
        }
    }

    pub async fn call(&self, prompt: &str) -> Result<LLMResponse, ClipboardError> {
        let api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| ClipboardError::ConfigError("OPENAI_API_KEY not set".to_string()))?;

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&serde_json::json!({
                "model": self.model,
                "messages": [{
                    "role": "user",
                    "content": prompt
                }]
            }))
            .send()
            .await
            .map_err(|e| ClipboardError::AIError(format!("Failed to send request: {}", e)))?;

        let response_json: serde_json::Value = response.json().await
            .map_err(|e| ClipboardError::AIError(format!("Failed to parse response: {}", e)))?;

        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| ClipboardError::AIError("Invalid AI response format".to_string()))?
            .to_string();

        let usage: TokenUsage = serde_json::from_value(response_json["usage"].clone())
            .map_err(|e| ClipboardError::AIError(format!("Failed to parse usage data: {}", e)))?;

        info!("Token usage: prompt={}, completion={}, total={}{}", 
            usage.prompt_tokens, 
            usage.completion_tokens, 
            usage.total_tokens,
            usage.completion_tokens_details.map_or(String::new(), |d| format!(
                ", details: reasoning={}, accepted={}, rejected={}", 
                d.reasoning_tokens, 
                d.accepted_prediction_tokens, 
                d.rejected_prediction_tokens
            ))
        );

        Ok(LLMResponse { content, usage })
    }

    pub async fn call_with_json_response<T: for<'de> Deserialize<'de>>(&self, prompt: &str) -> Result<(T, TokenUsage), ClipboardError> {
        let api_key = env::var("OPENAI_API_KEY")
            .map_err(|_| ClipboardError::ConfigError("OPENAI_API_KEY not set".to_string()))?;

        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&serde_json::json!({
                "model": self.model,
                "messages": [{
                    "role": "user",
                    "content": prompt
                }],
                "response_format": { "type": "json_object" }
            }))
            .send()
            .await
            .map_err(|e| ClipboardError::AIError(format!("Failed to send request: {}", e)))?;

        let response_json: serde_json::Value = response.json().await
            .map_err(|e| ClipboardError::AIError(format!("Failed to parse response: {}", e)))?;

        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| ClipboardError::AIError("Invalid AI response format".to_string()))?;

        let parsed: T = serde_json::from_str(content)
            .map_err(|e| ClipboardError::AIError(format!("Failed to parse JSON response: {}", e)))?;

        let usage: TokenUsage = serde_json::from_value(response_json["usage"].clone())
            .map_err(|e| ClipboardError::AIError(format!("Failed to parse usage data: {}", e)))?;

        info!("Token usage: prompt={}, completion={}, total={}{}", 
            usage.prompt_tokens, 
            usage.completion_tokens, 
            usage.total_tokens,
            usage.completion_tokens_details.map_or(String::new(), |d| format!(
                ", details: reasoning={}, accepted={}, rejected={}", 
                d.reasoning_tokens, 
                d.accepted_prediction_tokens, 
                d.rejected_prediction_tokens
            ))
        );

        Ok((parsed, usage))
    }
} 