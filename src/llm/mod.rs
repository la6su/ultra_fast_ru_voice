use tokio::sync::mpsc;
use tracing::{info, error};

#[derive(Debug)]
pub struct LlmRequest {
    pub request_id: uuid::Uuid,
    pub text: String,
}

#[derive(Debug)]
pub struct LlmResponse {
    pub request_id: uuid::Uuid,
    pub text: String,
    pub is_final: bool,
}

#[derive(Debug, Clone)]
pub enum LlmClientType {
    OpenAI { api_key: String },
    Local,
}

impl LlmClientType {
    pub async fn process(&self, request: LlmRequest) -> Result<Vec<LlmResponse>, LlmError> {
        match self {
            LlmClientType::OpenAI { api_key } => {
                let client = OpenAiClient::new(api_key.clone());
                client.process(request).await
            }
            LlmClientType::Local => {
                let client = LocalLlmClient::new();
                client.process(request).await
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LlmError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("API error: {0}")]
    Api(String),
    #[error("Request cancelled")]
    Cancelled,
}

pub struct OpenAiClient {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl OpenAiClient {
    pub fn new(api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key,
            model: "gpt-3.5-turbo".to_string(),
        }
    }
}

// Internal implementations
impl OpenAiClient {
    pub async fn process(&self, request: LlmRequest) -> Result<Vec<LlmResponse>, LlmError> {
        info!("Processing LLM request: {}", request.request_id);

        // Simple completion for now (streaming can be added later)
        let response = self.client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&serde_json::json!({
                "model": self.model,
                "messages": [
                    {
                        "role": "user",
                        "content": request.text
                    }
                ],
                "max_tokens": 100,
                "temperature": 0.7
            }))
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(LlmError::Api(error_text));
        }

        let response_json: serde_json::Value = response.json().await?;
        let content = response_json["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("Извините, я не понял.")
            .to_string();

        Ok(vec![LlmResponse {
            request_id: request.request_id,
            text: content,
            is_final: true,
        }])
    }
}

// Placeholder for future local LLM client
pub struct LocalLlmClient;

impl LocalLlmClient {
    pub fn new() -> Self {
        Self
    }

    pub async fn process(&self, _request: LlmRequest) -> Result<Vec<LlmResponse>, LlmError> {
        // Placeholder - would integrate with llama.cpp or similar
        Ok(vec![LlmResponse {
            request_id: uuid::Uuid::new_v4(),
            text: "Это заглушка для локальной модели LLM.".to_string(),
            is_final: true,
        }])
    }
}
