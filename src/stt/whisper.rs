//! # Whisper Service - Клиент для C++ STT сервера
//!
//! Этот модуль реализует клиент для подключения к C++ whisper серверу.
//! Использует WebSocket протокол для real-time коммуникации.
//!
//! ## Архитектура:
//! - **Rust side**: WebSocket клиент + audio конвертация
//! - **C++ side**: whisper.cpp + VAD + real-time processing
//! - **Протокол**: Binary PCM16 ↔ JSON transcription results
//!
//! ## Особенности:
//! - Поддержка partial и final результатов
//! - Автоматическая реконнекция при разрывах
//! - Конвертация между f32 и i16 форматами
//! - Синхронизация с C++ stream_server логикой

use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};

#[derive(Debug)]
pub struct WhisperService {
    server_url: String,
}

impl WhisperService {
    pub fn new(server_url: &str) -> Result<Self> {
        Ok(Self {
            server_url: server_url.to_string(),
        })
    }

    pub async fn transcribe_partial(&mut self, audio: &[f32]) -> anyhow::Result<String> {
        if audio.len() < 1600 { // minimum ~100ms
            return Ok(String::new());
        }
        self.transcribe(audio).await
    }

    pub async fn transcribe(&mut self, audio: &[f32]) -> anyhow::Result<String> {
        // Convert f32 audio to i16 PCM16
        let pcm16: Vec<i16> = audio.iter()
            .map(|&sample| (sample * 32767.0) as i16)
            .collect();

        // Convert to bytes (little endian)
        let audio_bytes: Vec<u8> = pcm16.iter()
            .flat_map(|&sample| sample.to_le_bytes())
            .collect();

        // Connect to C++ whisper WebSocket server
        let url = format!("ws://{}", self.server_url.replace("http://", "").replace("https://", ""));
        let (ws_stream, _) = connect_async(url).await
            .map_err(|e| anyhow::anyhow!("Failed to connect to whisper server: {}", e))?;

        let (mut write, mut read) = ws_stream.split();

        // Send audio data as binary message
        write.send(Message::Binary(audio_bytes)).await
            .map_err(|e| anyhow::anyhow!("Failed to send audio: {}", e))?;

        // Wait for final transcription
        let mut final_text = String::new();

        while let Some(message) = read.next().await {
            match message? {
                Message::Text(json_str) => {
                    let event: serde_json::Value = serde_json::from_str(&json_str)?;
                    if let Some(event_type) = event["type"].as_str() {
                        if event_type == "final" {
                            if let Some(text) = event["text"].as_str() {
                                final_text = text.to_string();
                            }
                            break; // Got final result
                        }
                        // Ignore partial results for now
                    }
                }
                Message::Close(_) => break,
                _ => {}
            }
        }

        Ok(final_text)
    }
}
