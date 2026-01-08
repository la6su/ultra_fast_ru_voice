//! # STT Pipeline - Единый orchestrator обработки речи
//!
//! Согласно promts/03_stt_pipeline_rules.md:
//! SttPipeline - ONLY place where VAD events, audio buffering, Whisper calls are orchestrated.
//!
//! ## Контракт:
//! AudioFrame → SttPipeline → Vec<SttEvent>
//!
//! ## Архитектура:
//! - Держит Vad для детекции речи
//! - Держит SpeechBuffer для буферизации
//! - Держит WhisperService для транскрибации
//! - Оркестрирует весь pipeline
//!
//! ## Важно:
//! - Единственный источник логики обработки речи
//! - Инкапсулирует VAD, Buffer, Whisper
//! - Возвращает события для внешних систем

use crate::audio::{
    speech_buffer::SpeechBuffer,
    types::{AudioFrame, VadEvent},
    vad::Vad,
};
use crate::stt::whisper::WhisperService;

#[derive(Debug, Clone)]
pub enum SttEvent {
    /// Частичная транскрибация (во время речи)
    Partial(String),
    /// Финальная транскрибация (после конца речи)
    Final(String),
}

/// SttPipeline - единый orchestrator обработки речи
pub struct SttPipeline {
    vad: Vad,
    buffer: SpeechBuffer,
    whisper: WhisperService,
}

impl SttPipeline {
    /// Создает новый pipeline
    pub fn new(whisper_server: &str) -> anyhow::Result<Self> {
        Ok(Self {
            vad: Vad::new(),
            buffer: SpeechBuffer::new(),
            whisper: WhisperService::new(whisper_server)?,
        })
    }

    /// Основной контракт pipeline
    /// Согласно promts/03_stt_pipeline_rules.md:
    /// SttPipeline - ONLY place for VAD events, audio buffering, Whisper calls
    pub async fn process_frame(&mut self, frame: AudioFrame) -> Vec<SttEvent> {
        let mut events = Vec::new();

        // ШАГ 1: VAD processing
        // VadEvent: SpeechStart, SpeechEnd (не SpeechFrame!)
        if let Some(vad_event) = self.vad.process(frame.clone()) {
            match vad_event {
                VadEvent::SpeechStart => {
                    // Начинаем буферизацию речи
                    self.buffer.start_speech();
                }
                VadEvent::SpeechEnd => {
                    // Заканчиваем буферизацию и транскрибируем
                    let audio_segment = self.buffer.end_speech();
                    if let Ok(text) = self.whisper.transcribe(&audio_segment).await {
                        events.push(SttEvent::Final(text));
                    }
                }
            }
        }

        // ШАГ 2: Продолжаем буферизацию во время речи
        if self.buffer.in_speech {
            self.buffer.push_frame(frame);
        }

        // ШАГ 3: Partial results (опционально)
        // Каждые ~200ms генерируем partial результат
        if self.buffer.in_speech && self.buffer.frame_count % 10 == 0 {
            if let Ok(text) = self.whisper.transcribe_partial(&self.buffer.current()).await {
                if !text.is_empty() {
                    events.push(SttEvent::Partial(text));
                }
            }
        }

        events
    }

    /// Сброс всего состояния pipeline
    pub fn reset(&mut self) {
        self.vad.reset();
        self.buffer.reset();
        // WhisperService сбрасывается автоматически
    }
}
