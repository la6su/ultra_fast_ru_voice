use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WsEvent {
    // VAD events
    VadStart,
    VadEnd,

    // STT events
    SttPartial { text: String },
    SttFinal { text: String },

    // LLM events
    LlmPartial { text: String },
    LlmFinal { text: String },

    // TTS events
    TtsStart,
    TtsAudio { data: String }, // Base64 encoded PCM16
    TtsEnd,
}
