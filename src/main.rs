//! # UltraFast RU Voice - Главный сервер голосового ассистента
//!
//! Этот файл является точкой входа для сервера голосового ассистента.
//! Сервер предоставляет WebSocket API для real-time обработки аудио.
//!
//! ## Архитектура:
//! - WebSocket сервер принимает PCM16 аудио
//! - Audio → VAD → Speech Buffer → Whisper STT → результаты
//! - Поддержка GPU ускорения и latency мониторинга

mod audio;
mod core;
mod llm;
mod stt;
mod tts;
mod utils;
mod ws;

use std::net::SocketAddr;

use audio::input::start_audio_input;
use audio::types::AudioFrame;
use core::state_machine::StateMachine;
use core::stt_pipeline::SttPipeline;
use futures::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use tracing::{info, error};
use uuid::Uuid;

use ws::handler::WsClient;
use ws::protocol::WsEvent;
use utils::{gpu, latency, health};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 Starting ultra_fast_ru_voice server");

    // GPU detection
    let gpu_info = gpu::GpuInfo::detect();
    let optimized_threads = gpu::optimize_whisper_threads(&gpu_info);
    info!("⚙️  Optimized for {} CPU threads", optimized_threads);

    // Latency tracking
    let mut latency_tracker = latency::LatencyTracker::new(1000);

    // Health checker
    let mut health_checker = health::HealthChecker::new(std::time::Duration::from_secs(30));

    // Read configuration
    let ws_host = std::env::var("WS_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let ws_port = std::env::var("WS_PORT").unwrap_or_else(|_| "8080".to_string());
    let whisper_server = std::env::var("WHISPER_CPP_SERVER").unwrap_or_else(|_| "127.0.0.1:9001".to_string());

    // Start WebSocket server
    let addr = format!("{}:{}", ws_host, ws_port).parse::<SocketAddr>()?;
    info!("WebSocket server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    let whisper_server_clone = whisper_server.clone();
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            let whisper_server = whisper_server_clone.clone();
            tokio::spawn(async move {
                let ws_stream = accept_async(stream).await.expect("Failed to accept");
                handle_connection(ws_stream, &whisper_server).await;
            });
        }
    });

    // Local microphone for testing
    let (tx, mut rx) = mpsc::channel::<AudioFrame>(32);
    start_audio_input(tx)?;

    let mut pipeline = SttPipeline::new(&whisper_server)?;

    while let Some(frame) = rx.recv().await {
        let events = pipeline.process_frame(frame).await;
        for ev in events {
            println!("{:?}", ev);
        }
    }

    Ok(())
}

async fn handle_connection(ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>, whisper_server: &str) {
    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    // Создаем чистого WS клиента (только транспорт, без бизнес-логики)
    let mut ws_client = match WsClient::new(whisper_server) {
        Ok(client) => client,
        Err(e) => {
            error!("Failed to create WS client: {}", e);
            return;
        }
    };

    // Latency tracking для performance monitoring
    let mut latency_tracker = latency::LatencyTracker::new(1000);

    info!("New WebSocket connection established");

    while let Some(msg) = ws_receiver.next().await {
        match msg {
            Ok(Message::Binary(data)) => {
                // Latency measurement: start timer
                let _latency_timer = latency::LatencyTimer::new(Some(&mut latency_tracker));

                // Binary PCM16 audio frame
                let pcm16: Vec<i16> = data
                    .chunks(2)
                    .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]))
                    .collect();

                // Обрабатываем через чистый WS handler
                let ws_events = ws_client.process_audio_frame(&pcm16).await;

                // Отправляем WsEvent'ы как JSON (согласно протоколу)
                for event in ws_events {
                    let json = serde_json::to_string(&event).unwrap();
                    if ws_sender.send(Message::Text(json)).await.is_err() {
                        error!("Failed to send WebSocket message");
                        return;
                    }
                }

                // Latency measurement: timer drops here, records duration
            }
            Ok(Message::Close(_)) => {
                info!("WebSocket connection closed");

                // Report final latency stats before closing
                latency_tracker.report();

                ws_client.reset();
                break;
            }
            _ => {
                // Игнорируем другие типы сообщений согласно протоколу
            }
        }
    }
}
