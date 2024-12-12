use serde::{Deserialize, Serialize};
use crate::errors::ClipboardError;
use reqwest::Client;
use std::env;
use tracing::info;
use std::ops::Add;
use std::collections::HashMap;
use once_cell::sync::Lazy;

pub static MODEL_PRICING: Lazy<HashMap<&'static str, ModelPricing>> = Lazy::new(|| {
    let mut m = HashMap::new();
    m.insert("gpt-4o", ModelPricing::new(2.50, 1.25, 10.0));
    m.insert("gpt-4o-2024-11-20", ModelPricing::new(2.50, 1.25, 10.0));
    m.insert("gpt-4o-2024-08-06", ModelPricing::new(2.50, 1.25, 10.0));
    m.insert("gpt-4o-2024-05-13", ModelPricing::new(5.00, 2.50, 15.0));
    m.insert("gpt-4o-mini", ModelPricing::new(0.150, 0.075, 0.600));
    m.insert("gpt-4o-mini-2024-07-18", ModelPricing::new(0.150, 0.075, 0.600));
    m.insert("o1-preview", ModelPricing::new(15.0, 7.50, 60.0));
    m.insert("o1-preview-2024-09-12", ModelPricing::new(15.0, 7.50, 60.0));
    m.insert("o1-mini", ModelPricing::new(3.0, 1.50, 12.0));
    m.insert("o1-mini-2024-09-12", ModelPricing::new(3.0, 1.50, 12.0));
    m
});

#[derive(Debug, Clone, Copy)]
pub struct ModelPricing {
    pub input_price: f64,    // per 1M tokens
    pub cached_price: f64,   // per 1M tokens
    pub output_price: f64,   // per 1M tokens
}

impl ModelPricing {
    pub fn new(input_price: f64, cached_price: f64, output_price: f64) -> Self {
        Self {
            input_price,
            cached_price,
            output_price,
        }
    }

    pub fn calculate_cost(&self, usage: &TokenUsage) -> f64 {
        let cached_cost = (usage.prompt_tokens_details.as_ref().map_or(0, |d| d.cached_tokens) as f64 / 1_000_000.0) * self.cached_price;
        let regular_input_cost = ((usage.prompt_tokens - usage.prompt_tokens_details.as_ref().map_or(0, |d| d.cached_tokens)) as f64 / 1_000_000.0) * self.input_price;
        let output_cost = (usage.completion_tokens as f64 / 1_000_000.0) * self.output_price;
        cached_cost + regular_input_cost + output_cost
    }
}

#[derive(Debug, Deserialize, Default, Clone, Copy)]
pub struct PromptTokenDetails {
    pub cached_tokens: u32,
}

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
    pub prompt_tokens_details: Option<PromptTokenDetails>,
    #[serde(default)]
    pub completion_tokens_details: Option<CompletionTokenDetails>,
}

impl TokenUsage {
    pub fn get_cost(&self, model: &str) -> Option<f64> {
        MODEL_PRICING.get(model).map(|pricing| pricing.calculate_cost(self))
    }

    pub fn format_details(&self, model: &str) -> String {
        let mut details = format!("prompt={}, completion={}, total={}", 
            self.prompt_tokens, 
            self.completion_tokens, 
            self.total_tokens
        );

        if let Some(pd) = self.prompt_tokens_details {
            details.push_str(&format!(", cached={}", pd.cached_tokens));
        }

        if let Some(cd) = self.completion_tokens_details {
            details.push_str(&format!(
                ", reasoning={}, accepted={}, rejected={}", 
                cd.reasoning_tokens,
                cd.accepted_prediction_tokens,
                cd.rejected_prediction_tokens
            ));
        }

        if let Some(cost) = self.get_cost(model) {
            details.push_str(&format!(", cost=${:.6}", cost));
        }

        details
    }
}

impl Add for TokenUsage {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            prompt_tokens: self.prompt_tokens + other.prompt_tokens,
            completion_tokens: self.completion_tokens + other.completion_tokens,
            total_tokens: self.total_tokens + other.total_tokens,
            prompt_tokens_details: match (self.prompt_tokens_details, other.prompt_tokens_details) {
                (Some(a), Some(b)) => Some(PromptTokenDetails {
                    cached_tokens: a.cached_tokens + b.cached_tokens,
                }),
                _ => None,
            },
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