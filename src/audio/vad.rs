//! # Voice Activity Detection (VAD) - Обнаружение активности голоса
//!
//! Этот модуль реализует обнаружение начала и конца речи на основе энергии сигнала.
//! Использует адаптивный порог с EMA (Exponential Moving Average) для учета шума.
//!
//! ## Алгоритм работы:
//! 1. Вычисляем RMS энергию для каждого фрейма
//! 2. Применяем EMA для сглаживания энергии
//! 3. Адаптируем порог шума когда нет речи
//! 4. Детектируем SpeechStart/SpeechEnd/SpeechFrame события
//!
//! ## Параметры:
//! - **start_multiplier**: Множитель порога для начала речи (обычно 3.0)
//! - **end_multiplier**: Множитель порога для конца речи (обычно 1.5)
//! - **hangover_frames**: Задержка перед SpeechEnd (предотвращает фрагментацию)
//! - **ema_alpha**: Коэффициент сглаживания EMA (0.1 = быстрая адаптация)
//!
//! ## Особенности:
//! - Адаптивный порог шума для разных условий
//! - Hangover механизм для стабильности
//! - Cooldown период после SpeechEnd

use crate::audio::types::{AudioFrame, VadEvent};

pub struct Vad {
    // state
    in_speech: bool,
    silence_frames: usize,
    cooldown: usize,

    // energy tracking
    noise_floor: f32,
    energy_ema: f32,

    // params
    start_multiplier: f32,
    end_multiplier: f32,
    hangover_frames: usize,
    ema_alpha: f32,
}

impl Vad {
    pub fn new() -> Self {
        Self {
            in_speech: false,
            silence_frames: 0,
            cooldown: 0,

            noise_floor: 0.0005,
            energy_ema: 0.0,

            // 🎯 эталонные параметры для RU речи
            start_multiplier: 3.0,
            end_multiplier: 1.5,
            hangover_frames: 8, // ~160ms
            ema_alpha: 0.1,
        }
    }

    pub fn reset(&mut self) {
        self.in_speech = false;
        self.silence_frames = 0;
        self.cooldown = 0;
        self.energy_ema = 0.0;
    }

    /// Обрабатывает аудио фрейм и детектирует начало/конец речи
    /// Согласно promts/02_audio_and_vad_contract.md:
    /// - Принимает AudioFrame
    /// - Возвращает VadEvent (SpeechStart/SpeechEnd)
    /// - НЕ накапливает аудио, НЕ вызывает STT, НЕ знает про WebSocket
    pub fn process(&mut self, frame: AudioFrame) -> Option<VadEvent> {
        // cooldown после SpeechEnd — критично для стабильности
        if self.cooldown > 0 {
            self.cooldown -= 1;
            return None;
        }

        let energy = rms_energy(&frame.samples);

        // EMA сглаживание энергии
        self.energy_ema = self.ema_alpha * energy + (1.0 - self.ema_alpha) * self.energy_ema;

        // адаптация noise floor (только вне речи)
        if !self.in_speech {
            self.noise_floor = 0.99 * self.noise_floor + 0.01 * self.energy_ema;
        }

        let start_threshold = self.noise_floor * self.start_multiplier;
        let end_threshold = self.noise_floor * self.end_multiplier;

        if !self.in_speech {
            if self.energy_ema > start_threshold {
                self.in_speech = true;
                self.silence_frames = 0;
                return Some(VadEvent::SpeechStart);
            }
            return None;
        }

        // IN SPEECH: проверяем конец речи
        if self.energy_ema < end_threshold {
            self.silence_frames += 1;
            if self.silence_frames >= self.hangover_frames {
                self.in_speech = false;
                self.silence_frames = 0;
                self.energy_ema = 0.0;
                self.cooldown = 3; // ~60ms reset window
                return Some(VadEvent::SpeechEnd);
            }
        } else {
            self.silence_frames = 0;
        }

        None // Продолжаем речь, без событий
    }
}

fn rms_energy(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f32 = samples.iter().map(|s| s * s).sum();
    (sum / samples.len() as f32).sqrt()
}
