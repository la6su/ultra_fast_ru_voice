//! # Speech Buffer - Буфер аудио сегментов
//!
//! Этот модуль управляет буферизацией аудио во время речи.
//! Реализует pre-roll buffer для захвата начала речи.
//!
//! ## Концепция:
//! - **Pre-roll**: Несколько фреймов ДО обнаружения речи VAD
//! - **Active**: Аудио во время активной речи
//! - **Segmentation**: Автоматическое разделение на речевые сегменты
//!
//! ## Алгоритм работы:
//! 1. Всегда храним PRE_ROLL_FRAMES в pre_roll буфере
//! 2. При SpeechStart переносим pre_roll → active
//! 3. Добавляем новые фреймы в active
//! 4. При SpeechEnd возвращаем полный сегмент и очищаем буферы
//!
//! ## Параметры:
//! - **PRE_ROLL_FRAMES**: Количество фреймов захвата перед речью (обычно 5 = 100ms)

use crate::audio::types::AudioFrame;

const PRE_ROLL_FRAMES: usize = 5;

pub struct SpeechBuffer {
    pub pre_roll: Vec<AudioFrame>,
    pub active: Vec<AudioFrame>,
    pub in_speech: bool,
    pub frame_count: usize, // Для partial results timing
}

impl SpeechBuffer {
    pub fn new() -> Self {
        Self {
            pre_roll: Vec::new(),
            active: Vec::new(),
            in_speech: false,
            frame_count: 0,
        }
    }

    /// Добавить фрейм в буфер
    pub fn push(&mut self, frame: AudioFrame) {
        if self.in_speech {
            self.active.push(frame);
        } else {
            // Pre-roll buffer для захвата начала речи
            self.pre_roll.push(frame);
            if self.pre_roll.len() > PRE_ROLL_FRAMES {
                self.pre_roll.remove(0);
            }
        }
    }

    /// Начать запись речи (перенести pre-roll в active)
    /// Вызывается при VadEvent::SpeechStart
    pub fn start_speech(&mut self) {
        self.in_speech = true;
        self.frame_count = 0;
        self.active.extend(self.pre_roll.drain(..));
    }

    /// Добавить фрейм в активную запись
    /// Вызывается во время речи (после start_speech, до end_speech)
    pub fn push_frame(&mut self, frame: AudioFrame) {
        if self.in_speech {
            self.active.push(frame);
            self.frame_count += 1;
        }
    }

    /// Закончить запись речи и вернуть накопленный сегмент
    /// Вызывается при VadEvent::SpeechEnd
    pub fn end_speech(&mut self) -> Vec<f32> {
        self.in_speech = false;

        let mut samples = Vec::new();
        for frame in self.active.drain(..) {
            samples.extend(frame.samples);
        }

        self.pre_roll.clear();
        samples
    }

    /// Получить текущий буфер для partial transcription
    pub fn current(&self) -> Vec<f32> {
        let mut samples = Vec::new();
        for frame in &self.active {
            samples.extend(&frame.samples);
        }
        samples
    }

    /// Сбросить все состояния
    pub fn reset(&mut self) {
        self.pre_roll.clear();
        self.active.clear();
        self.in_speech = false;
        self.frame_count = 0;
    }
}
