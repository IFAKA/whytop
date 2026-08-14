use crate::{process::ProcessSnapshot, prompt};
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use futures_util::StreamExt;
use reqwest::Client;
use serde::Deserialize;
use tokio::sync::mpsc::Sender;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone)]
pub enum AiEvent {
    Started,
    Token(String),
    Finished { ttft_ms: u128, tokens: usize },
    Error(String),
}

#[async_trait]
pub trait AiEngine: Send + Sync {
    async fn explain(
        &self,
        snapshot: ProcessSnapshot,
        question: String,
        events: Sender<AiEvent>,
        cancel: CancellationToken,
    ) -> Result<()>;
    fn label(&self) -> &'static str;
}

#[derive(Clone)]
pub struct OpenAiLocalEngine {
    client: Client,
    base_url: String,
    configured_model: String,
    label: &'static str,
}

impl OpenAiLocalEngine {
    pub fn from_env() -> Self {
        let (base_url, configured_model, label) = if cfg!(target_os = "macos") {
            (
                std::env::var("WHYTOP_MLX_URL")
                    .unwrap_or_else(|_| "http://127.0.0.1:8000/v1".into()),
                std::env::var("WHYTOP_MLX_MODEL").unwrap_or_else(|_| "nail-qwen3.6-35b-a3b".into()),
                "Rapid-MLX",
            )
        } else {
            (
                std::env::var("WHYTOP_LLAMA_URL")
                    .unwrap_or_else(|_| "http://127.0.0.1:8080/v1".into()),
                std::env::var("WHYTOP_LLAMA_MODEL")
                    .unwrap_or_else(|_| "openbmb/MiniCPM5-1B-GGUF".into()),
                "llama-server",
            )
        };
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').into(),
            configured_model,
            label,
        }
    }

    async fn model(&self) -> Result<String> {
        #[derive(Deserialize)]
        struct Models {
            data: Vec<Model>,
        }
        #[derive(Deserialize)]
        struct Model {
            id: String,
        }
        let response = self
            .client
            .get(format!("{}/models", self.base_url))
            .send()
            .await?
            .error_for_status()?;
        let models: Models = response.json().await?;
        if models.data.iter().any(|m| m.id == self.configured_model) {
            Ok(self.configured_model.clone())
        } else {
            let available = models
                .data
                .iter()
                .map(|model| model.id.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            Err(anyhow!(
                "wrong model server model: expected {}, available: {}",
                self.configured_model,
                if available.is_empty() {
                    "none"
                } else {
                    &available
                }
            ))
        }
    }
}

#[async_trait]
impl AiEngine for OpenAiLocalEngine {
    async fn explain(
        &self,
        snapshot: ProcessSnapshot,
        question: String,
        events: Sender<AiEvent>,
        cancel: CancellationToken,
    ) -> Result<()> {
        let started = std::time::Instant::now();
        let _ = events.send(AiEvent::Started).await;
        let model = self.model().await?;
        let body = serde_json::json!({"model": model, "stream": true, "temperature": 0.1, "messages": [{"role":"user", "content": prompt::build(&snapshot, &question)}]});
        let response = self
            .client
            .post(format!("{}/chat/completions", self.base_url))
            .json(&body)
            .send()
            .await?
            .error_for_status()?;
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();
        let mut count = 0usize;
        let mut ttft = None;
        while let Some(chunk) = tokio::select! { value = stream.next() => value, _ = cancel.cancelled() => return Err(anyhow!("cancelled")) }
        {
            buffer.push_str(&String::from_utf8_lossy(&chunk?));
            while let Some(pos) = buffer.find('\n') {
                let line = buffer.drain(..=pos).collect::<String>();
                let data = line.trim();
                if let Some(json) = data.strip_prefix("data:").map(str::trim) {
                    if json == "[DONE]" {
                        continue;
                    }
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json) {
                        if let Some(token) = value["choices"][0]["delta"]["content"].as_str() {
                            if !token.is_empty() {
                                ttft.get_or_insert(started.elapsed().as_millis());
                                count += 1;
                                let _ = events.send(AiEvent::Token(token.into())).await;
                            }
                        }
                    }
                }
            }
        }
        let _ = events
            .send(AiEvent::Finished {
                ttft_ms: ttft.unwrap_or(started.elapsed().as_millis()),
                tokens: count,
            })
            .await;
        Ok(())
    }
    fn label(&self) -> &'static str {
        self.label
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn configured_platform_defaults_are_distinct() {
        assert!("openbmb/MiniCPM5-1B-MLX".contains("MLX"));
        assert!("openbmb/MiniCPM5-1B-GGUF".contains("GGUF"));
    }
}
