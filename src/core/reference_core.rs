//! # Reference Minimal Core - Ядро голосового ассистента
//!
//! Этот модуль содержит **бизнес-логику** обработки аудио БЕЗ транспорта.
//! Здесь реализован основной pipeline обработки речи.
//!
//! ## Фиксированный контракт обработки:
//! ```
//! AudioFrame → Vad::process → VadEvent → SpeechBuffer::on_event → WhisperService::transcribe → Vec<SttEvent>
//! ```
//!
//! ## Компоненты:
//! - **Vad**: Обнаружение начала/конца речи
//! - **SpeechBuffer**: Буферизация и сегментация аудио
//! - **WhisperService**: Транскрибация через C++ сервер
//! - **ReferencePipeline**: Оркестрация всех компонентов
//!
//! ## Важно:
//! - Чистая логика без WebSocket/HTTP
//! - Тестируемый и изолированный код
//! - Синхронизирован с C++ stream_server

use crate::audio::{
    speech_buffer::SpeechBuffer,
    types::{AudioFrame, VadEvent},
    vad::Vad,
};
use crate::stt::whisper::WhisperService;

#[derive(Debug, Clone)]
pub enum SttEvent {
    Partial(String),
    Final(String),
}

/// Reference pipeline: чистая логика без транспорта
pub struct ReferencePipeline {
    pub vad: Vad,
    pub buffer: SpeechBuffer,
    pub whisper: WhisperService,
}

impl ReferencePipeline {
    pub fn new(whisper_server: &str) -> anyhow::Result<Self> {
        Ok(Self {
            vad: Vad::new(),
            buffer: SpeechBuffer::new(),
            whisper: WhisperService::new(whisper_server)?,
        })
    }

    /// Reference contract: AudioFrame → Vec<SttEvent>
    pub async fn process_frame(&mut self, frame: AudioFrame) -> Vec<SttEvent> {
        let mut events = Vec::new();

        // Шаг 1: VAD processing
        if let Some(vad_event) = self.vad.process(frame.clone()) {
            // Шаг 2: Buffer lifecycle
            match vad_event {
                VadEvent::SpeechStart => {
                    self.buffer.start_speech();
                }
                VadEvent::SpeechEnd => {
                    // Шаг 3: Transcription
                    let segment = self.buffer.end_speech();
                    if let Ok(text) = self.whisper.transcribe(&segment).await {
                        events.push(SttEvent::Final(text));
                    }
                }
            }
        }

        // Шаг 4: Partial results (optional)
        if self.buffer.in_speech {
            self.buffer.frame_count += 1;
            if self.buffer.frame_count % 10 == 0 { // ~200ms
                if let Ok(text) = self.whisper.transcribe_partial(&self.buffer.current()).await {
                    if !text.is_empty() {
                        events.push(SttEvent::Partial(text));
                    }
                }
            }
        }

        events
    }

    pub fn reset(&mut self) {
        self.buffer.reset();
        self.vad.reset();
    }
}
