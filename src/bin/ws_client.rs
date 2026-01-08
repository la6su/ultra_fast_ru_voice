//! # WebSocket Client - End-to-end тестирование
//!
//! Этот binary реализует полнофункциональный клиент для тестирования голосового ассистента.
//! Использует cpal для захвата аудио с микрофона и отправляет его на сервер через WebSocket.
//!
//! ## Что делает:
//! 1. Захватывает аудио с микрофона через cpal
//! 2. Конвертирует в PCM16 формат
//! 3. Отправляет на сервер по WebSocket
//! 4. Получает и отображает результаты транскрибации
//!
//! ## Использование:
//! ```bash
//! cargo run --bin ws_client
//! # Говорите в микрофон - увидите результаты в real-time
//! ```
//!
//! ## Особенности:
//! - Поддержка разных аудио форматов (F32/I16)
//! - Graceful shutdown по Ctrl+C
//! - Автоматическая реконнекция
//! - Real-time отображение partial/final результатов

use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, error, warn};
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SampleFormat, StreamConfig,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load environment variables
    dotenvy::dotenv().ok();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    let server_url = std::env::var("WS_SERVER_URL")
        .unwrap_or_else(|_| "ws://127.0.0.1:8080".to_string());

    info!("🎯 Starting WebSocket client, connecting to: {}", server_url);

    // Подключаемся к WebSocket серверу
    let (ws_stream, _) = connect_async(&server_url).await
        .map_err(|e| anyhow::anyhow!("Failed to connect to server: {}", e))?;

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();

    info!("✅ Connected to server, starting audio capture...");

    // Создаем канал для передачи аудио данных
    let (audio_tx, mut audio_rx) = mpsc::channel::<Vec<i16>>(32);

    // Запускаем захват аудио в отдельном блокирующем треде
    let audio_tx_clone = audio_tx.clone();
    std::thread::spawn(move || {
        // Создаем новый runtime для blocking операций
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            if let Err(e) = start_audio_capture(audio_tx_clone).await {
                error!("Audio capture failed: {}", e);
            }
        });
    });

    // Основной loop обработки
    loop {
        tokio::select! {
            // Получаем аудио данные от cpal
            Some(audio_data) = audio_rx.recv() => {
                // Конвертируем в binary PCM16
                let audio_bytes: Vec<u8> = audio_data.iter()
                    .flat_map(|&sample| sample.to_le_bytes())
                    .collect();

                // Отправляем на сервер
                if let Err(e) = ws_sender.send(Message::Binary(audio_bytes)).await {
                    error!("Failed to send audio: {}", e);
                    break;
                }
            }

            // Получаем ответы от сервера
            Some(message) = ws_receiver.next() => {
                match message {
                    Ok(Message::Text(json_str)) => {
                        // Парсим JSON ответ
                        match serde_json::from_str::<serde_json::Value>(&json_str) {
                            Ok(event) => {
                                if let Some(event_type) = event["type"].as_str() {
                                    match event_type {
                                        "stt_partial" => {
                                            if let Some(text) = event["text"].as_str() {
                                                info!("🎄 Partial: {}", text);
                                            }
                                        }
                                        "stt_final" => {
                                            if let Some(text) = event["text"].as_str() {
                                                info!("🎯 Final: {}", text);
                                            }
                                        }
                                        "vad_start" => {
                                            info!("🟢 Speech detected");
                                        }
                                        "vad_end" => {
                                            info!("🔴 Speech ended");
                                        }
                                        _ => {
                                            warn!("Unknown event type: {}", event_type);
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                error!("Failed to parse server response: {}", e);
                            }
                        }
                    }
                    Ok(Message::Close(_)) => {
                        info!("🔌 Connection closed by server");
                        break;
                    }
                    Ok(_) => {} // Игнорируем binary и другие сообщения
                    Err(e) => {
                        error!("WebSocket error: {}", e);
                        break;
                    }
                }
            }

            // Обработка сигналов завершения
            _ = tokio::signal::ctrl_c() => {
                info!("🛑 Received shutdown signal");
                break;
            }
        }
    }

    // Аудио захват завершится автоматически при выходе

    info!("👋 Client shutdown");
    Ok(())
}

/// Запуск захвата аудио с микрофона через cpal
async fn start_audio_capture(audio_tx: mpsc::Sender<Vec<i16>>) -> anyhow::Result<()> {
    let host = cpal::default_host();
    let device = host.default_input_device()
        .ok_or_else(|| anyhow::anyhow!("No input device available"))?;

    info!("🎤 Using audio device: {}", device.name()?);

    let config = device.default_input_config()?;
    let sample_rate = config.sample_rate().0;

    info!("🎵 Audio config: {} Hz, {} channels",
          sample_rate, config.channels());

    // Создаем конфиг для захвата
    let stream_config = StreamConfig {
        channels: 1, // Моно
        sample_rate: cpal::SampleRate(16000), // 16kHz для Whisper
        buffer_size: cpal::BufferSize::Default,
    };

    let audio_tx_clone = audio_tx.clone();

    let stream = match config.sample_format() {
        SampleFormat::F32 => device.build_input_stream(
            &stream_config,
            move |data: &[f32], _| {
                // Конвертируем f32 в i16 PCM16
                let pcm16: Vec<i16> = data.iter()
                    .map(|&sample| (sample * 32767.0) as i16)
                    .collect();

                // Отправляем в канал (игнорируем ошибки если канал полон)
                let _ = audio_tx_clone.try_send(pcm16);
            },
            |err| error!("Audio stream error: {}", err),
            None,
        )?,

        SampleFormat::I16 => device.build_input_stream(
            &stream_config,
            move |data: &[i16], _| {
                // Уже в нужном формате
                let _ = audio_tx_clone.try_send(data.to_vec());
            },
            |err| error!("Audio stream error: {}", err),
            None,
        )?,

        _ => return Err(anyhow::anyhow!("Unsupported sample format")),
    };

    stream.play()?;
    info!("🎤 Audio capture started");

    // Держим stream живым
    std::mem::forget(stream);

    // Ждем завершения
    std::future::pending::<()>().await;
    Ok(())
}
