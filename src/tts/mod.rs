use std::process::Stdio;
use tokio::process::Command;
use tracing::{info, error};

#[derive(Debug)]
pub struct TtsRequest {
    pub request_id: uuid::Uuid,
    pub text: String,
    pub language: String,
}

#[derive(Debug)]
pub struct TtsResponse {
    pub request_id: uuid::Uuid,
    pub audio_data: Vec<u8>, // PCM16 audio bytes
    pub sample_rate: u32,
    pub is_final: bool,
}

#[derive(Debug, Clone)]
pub enum TtsClientType {
    Silero,
    Local,
}

impl TtsClientType {
    pub async fn process(&self, request: TtsRequest) -> Result<Vec<TtsResponse>, TtsError> {
        match self {
            TtsClientType::Silero => {
                let client = SileroClient::new();
                client.process(request).await
            }
            TtsClientType::Local => {
                let client = LocalTtsClient::new();
                client.process(request).await
            }
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum TtsError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Process error: {0}")]
    Process(String),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),
    #[error("Request cancelled")]
    Cancelled,
}

pub struct SileroClient {
    python_script: String,
}

impl SileroClient {
    pub fn new() -> Self {
        Self {
            python_script: r#"
import sys
import json
import torch
from silero_api import SileroTTS

def generate_speech(text, language='ru'):
    try:
        # Load Silero model
        model = SileroTTS(language=language)
        
        # Generate audio
        audio = model.tts(text)
        
        # Convert to PCM16 bytes
        audio_int16 = (audio * 32767).clamp(-32768, 32767).to(torch.int16)
        audio_bytes = audio_int16.numpy().tobytes()
        
        return audio_bytes, 24000  # Silero default sample rate
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        return None, None

if __name__ == "__main__":
    data = json.loads(sys.stdin.read())
    text = data['text']
    language = data.get('language', 'ru')
    
    audio_bytes, sample_rate = generate_speech(text, language)
    
    if audio_bytes is not None:
        result = {
            'success': True,
            'audio_data': audio_bytes.hex(),
            'sample_rate': sample_rate
        }
    else:
        result = {'success': False, 'error': 'TTS generation failed'}
    
    print(json.dumps(result))
"#.to_string(),
        }
    }
}

// Internal implementations
impl SileroClient {
    pub async fn process(&self, request: TtsRequest) -> Result<Vec<TtsResponse>, TtsError> {
        info!("Processing TTS request: {}", request.request_id);

        // Prepare input JSON
        let input_data = serde_json::json!({
            "text": request.text,
            "language": request.language
        });

        // Spawn Python subprocess
        let mut child = Command::new("python3")
            .arg("-c")
            .arg(&self.python_script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        // Send input data
        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            let input_json = serde_json::to_string(&input_data)?;
            stdin.write_all(input_json.as_bytes()).await?;
            stdin.flush().await?;
            drop(stdin); // Close stdin to signal end of input
        }

        // Wait for completion
        let output = child.wait_with_output().await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            error!("TTS process failed: {}", stderr);
            return Err(TtsError::Process(stderr.to_string()));
        }

        // Parse output
        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: serde_json::Value = serde_json::from_str(&stdout.trim())?;

        if result["success"].as_bool().unwrap_or(false) {
            let audio_hex = result["audio_data"].as_str().unwrap_or("");
            let audio_data = hex::decode(audio_hex)?;
            let sample_rate = result["sample_rate"].as_u64().unwrap_or(24000) as u32;

            Ok(vec![TtsResponse {
                request_id: request.request_id,
                audio_data,
                sample_rate,
                is_final: true,
            }])
        } else {
            let error_msg = result["error"].as_str().unwrap_or("Unknown TTS error");
            Err(TtsError::Process(error_msg.to_string()))
        }
    }
}

// Placeholder for future direct Rust TTS client
pub struct LocalTtsClient;

impl LocalTtsClient {
    pub fn new() -> Self {
        Self
    }

    pub async fn process(&self, _request: TtsRequest) -> Result<Vec<TtsResponse>, TtsError> {
        // Placeholder - would integrate with Rust TTS library
        Ok(vec![TtsResponse {
            request_id: uuid::Uuid::new_v4(),
            audio_data: vec![], // Empty audio
            sample_rate: 24000,
            is_final: true,
        }])
    }
}
