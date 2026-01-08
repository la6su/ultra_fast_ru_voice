#[derive(Debug, Clone)]
pub struct AudioFrame {
    /// PCM samples [-1.0, 1.0]
    pub samples: Vec<f32>,
    /// sample rate (e.g. 16000)
    pub sample_rate: u32,
}
impl AudioFrame {
    pub fn from_pcm16(samples: &[i16], sample_rate: u32) -> Self {
        let samples = samples.iter().map(|s| *s as f32 / 32768.0).collect();

        Self {
            samples,
            sample_rate,
        }
    }
}
#[derive(Debug, Clone)]
pub enum VadEvent {
    /// Начало речи обнаружено
    SpeechStart,
    /// Конец речи обнаружен
    SpeechEnd,
}
#[derive(Debug)]
pub struct SpeechSegment {
    pub samples: Vec<f32>,
    pub sample_rate: u32,
}
