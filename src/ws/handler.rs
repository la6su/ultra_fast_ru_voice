//! # WebSocket Handler - Транспортный слой
//!
//! Этот модуль реализует **чистый транспорт** без бизнес-логики.
//! Он отвечает только за конвертацию между WebSocket сообщениями и внутренними событиями.
//!
//! ## Протокол (согласно promts/04_ws_protocol_rules.md):
//! - **Входящие**: Binary PCM16 audio frames (16kHz, mono, little-endian)
//! - **Исходящие**: JSON WsEvent'ы (stt.partial/final, vad.start, vad.end)
//!
//! ## Принцип работы:
//! 1. Получаем binary audio от клиента
//! 2. Конвертируем в AudioFrame
//! 3. Отправляем в SttPipeline (single source of truth)
//! 4. Получаем SttEvent'ы обратно
//! 5. Конвертируем в WsEvent'ы согласно протоколу
//!
//! ## Важно:
//! - Никакой VAD/STT логики здесь!
//! - Только транспорт и конвертация типов
//! - Полная изоляция от бизнес-правил

use crate::core::stt_pipeline::{SttPipeline, SttEvent};
use crate::audio::types::AudioFrame;
use crate::ws::protocol::WsEvent;

pub struct WsClient {
    pipeline: SttPipeline,
}

impl WsClient {
    /// Создает нового WS клиента с STT pipeline
    pub fn new(whisper_server: &str) -> anyhow::Result<Self> {
        Ok(Self {
            pipeline: SttPipeline::new(whisper_server)?,
        })
    }

    /// Обрабатывает входящий binary audio frame
    /// Согласно promts/04_ws_protocol_rules.md:
    /// Binary PCM16 → AudioFrame → SttPipeline → WsEvent[]
    pub async fn process_audio_frame(&mut self, pcm16: &[i16]) -> Vec<WsEvent> {
        // Конвертируем binary PCM16 в AudioFrame
        let frame = AudioFrame::from_pcm16(pcm16, 16_000);

        // Отправляем в pipeline (single source of truth)
        let stt_events = self.pipeline.process_frame(frame).await;

        // Конвертируем SttEvent в WsEvent согласно протоколу
        stt_events.into_iter().map(|event| match event {
            SttEvent::Partial(text) => WsEvent::SttPartial { text },
            SttEvent::Final(text) => WsEvent::SttFinal { text },
        }).collect()
    }

    /// Reset pipeline state (для нового клиента)
    pub fn reset(&mut self) {
        self.pipeline.reset();
    }
}
