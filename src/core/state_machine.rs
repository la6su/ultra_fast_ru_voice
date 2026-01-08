use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::sync::mpsc;
use tracing::{info, warn, error};
use uuid::Uuid;

use crate::ws::protocol::WsEvent;
use crate::llm::{LlmClientType, LlmRequest};
use crate::tts::{TtsClientType, TtsRequest};

#[derive(Debug, Clone, PartialEq)]
pub enum AssistantState {
    Idle,
    Listening { request_id: Uuid, start_time: std::time::Instant },
    SpeechDetected { request_id: Uuid },
    Thinking { request_id: Uuid },
    Speaking { request_id: Uuid },
    Interrupted { request_id: Uuid },
}

#[derive(Debug, Clone)]
pub struct RequestContext {
    pub id: Uuid,
    pub audio_start: Option<std::time::Instant>,
    pub stt_start: Option<std::time::Instant>,
    pub stt_end: Option<std::time::Instant>,
    pub llm_start: Option<std::time::Instant>,
    pub tts_start: Option<std::time::Instant>,
    pub cancelled: bool,
}

impl RequestContext {
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            audio_start: None,
            stt_start: None,
            stt_end: None,
            llm_start: None,
            tts_start: None,
            cancelled: false,
        }
    }

    pub fn latency_ms(&self) -> Option<u128> {
        if let (Some(audio_start), Some(tts_start)) = (self.audio_start, self.tts_start) {
            Some(tts_start.duration_since(audio_start).as_millis())
        } else {
            None
        }
    }
}

#[derive(Clone)]
pub struct StateMachine {
    current_state: AssistantState,
    requests: Arc<RwLock<HashMap<Uuid, RequestContext>>>,
    llm_client: Option<LlmClientType>,
    tts_client: Option<TtsClientType>,
    event_sender: mpsc::UnboundedSender<WsEvent>,
}

impl StateMachine {
    pub fn new() -> Self {
        let (event_sender, _) = mpsc::unbounded_channel();
        Self {
            current_state: AssistantState::Idle,
            requests: Arc::new(RwLock::new(HashMap::new())),
            llm_client: None,
            tts_client: None,
            event_sender,
        }
    }

    pub fn with_clients(
        mut self,
        llm_client: Option<LlmClientType>,
        tts_client: Option<TtsClientType>,
        event_sender: mpsc::UnboundedSender<WsEvent>,
    ) -> Self {
        self.llm_client = llm_client;
        self.tts_client = tts_client;
        self.event_sender = event_sender;
        self
    }

    async fn process_llm(&self, request_id: Uuid, text: String) {
        if let Some(llm_client) = &self.llm_client {
            let request = LlmRequest { request_id, text };
            match llm_client.process(request).await {
                Ok(responses) => {
                    for response in responses {
                        if response.is_final {
                            // Send final LLM result to trigger state transition
                            let _ = self.event_sender.send(WsEvent::LlmFinal {
                                text: response.text,
                            });
                        } else {
                            let _ = self.event_sender.send(WsEvent::LlmPartial {
                                text: response.text,
                            });
                        }
                    }
                }
                Err(e) => {
                    error!("LLM processing failed: {}", e);
                    // Fallback response
                    let _ = self.event_sender.send(WsEvent::LlmFinal {
                        text: "Извините, я не понял запрос.".to_string(),
                    });
                }
            }
        }
    }

    async fn process_tts(&self, request_id: Uuid, text: String) {
        if let Some(tts_client) = &self.tts_client {
            let request = TtsRequest {
                request_id,
                text,
                language: "ru".to_string(),
            };
            match tts_client.process(request).await {
                Ok(responses) => {
                    let _ = self.event_sender.send(WsEvent::TtsStart);
                    for response in responses {
                        if response.is_final {
                            let audio_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &response.audio_data);
                            let _ = self.event_sender.send(WsEvent::TtsAudio { data: audio_b64 });
                            let _ = self.event_sender.send(WsEvent::TtsEnd);
                        }
                    }
                }
                Err(e) => {
                    error!("TTS processing failed: {}", e);
                    let _ = self.event_sender.send(WsEvent::TtsEnd);
                }
            }
        }
    }

    pub async fn transition(&mut self, event: &WsEvent) -> Vec<WsEvent> {
        let mut outgoing_events = Vec::new();

        match (&self.current_state.clone(), event) {
            // Idle → Listening on VadStart
            (AssistantState::Idle, WsEvent::VadStart) => {
                let request_id = Uuid::new_v4();
                let context = RequestContext {
                    id: request_id,
                    audio_start: Some(std::time::Instant::now()),
                    ..RequestContext::new()
                };

                self.requests.write().await.insert(request_id, context);
                self.current_state = AssistantState::Listening {
                    request_id,
                    start_time: std::time::Instant::now(),
                };

                outgoing_events.push(WsEvent::VadStart);
                info!("Assistant: Idle → Listening (request_id: {})", request_id);
            }

            // Listening → SpeechDetected (could add logic here)
            (AssistantState::Listening { request_id, .. }, WsEvent::VadEnd) => {
                self.current_state = AssistantState::SpeechDetected { request_id: *request_id };
                outgoing_events.push(WsEvent::VadEnd);
                info!("Assistant: Listening → SpeechDetected (request_id: {})", request_id);
            }

            // SpeechDetected → Thinking on STT Final
            (AssistantState::SpeechDetected { request_id }, WsEvent::SttFinal { text }) => {
                if let Some(ctx) = self.requests.write().await.get_mut(request_id) {
                    ctx.stt_end = Some(std::time::Instant::now());
                    ctx.llm_start = Some(std::time::Instant::now());
                }

                self.current_state = AssistantState::Thinking { request_id: *request_id };

                // Trigger LLM processing
                let state_machine_clone = self.clone();
                let request_id_clone = *request_id;
                let text_clone = text.clone();
                tokio::spawn(async move {
                    state_machine_clone.process_llm(request_id_clone, text_clone).await;
                });

                info!("Assistant: SpeechDetected → Thinking (request_id: {}, text: '{}')", request_id, text);
            }

            // Thinking → Speaking on LLM completion (for now, immediate)
            (AssistantState::Thinking { request_id }, WsEvent::LlmFinal { text }) => {
                if let Some(ctx) = self.requests.write().await.get_mut(request_id) {
                    ctx.tts_start = Some(std::time::Instant::now());
                }

                self.current_state = AssistantState::Speaking { request_id: *request_id };

                // Trigger TTS processing
                let state_machine_clone = self.clone();
                let request_id_clone = *request_id;
                let text_clone = text.clone();
                tokio::spawn(async move {
                    state_machine_clone.process_tts(request_id_clone, text_clone).await;
                });

                info!("Assistant: Thinking → Speaking (request_id: {})", request_id);
            }

            // Any state → Interrupted on new VadStart (cancel previous)
            (_, WsEvent::VadStart) if !matches!(self.current_state, AssistantState::Idle) => {
                // Cancel current request
                if let AssistantState::Listening { request_id, .. } |
                     AssistantState::SpeechDetected { request_id } |
                     AssistantState::Thinking { request_id } |
                     AssistantState::Speaking { request_id } = &self.current_state {
                    if let Some(ctx) = self.requests.write().await.get_mut(request_id) {
                        ctx.cancelled = true;
                    }
                    warn!("Request cancelled: {}", request_id);
                }

                // Start new request
                let new_request_id = Uuid::new_v4();
                let context = RequestContext {
                    id: new_request_id,
                    audio_start: Some(std::time::Instant::now()),
                    ..RequestContext::new()
                };

                self.requests.write().await.insert(new_request_id, context);
                self.current_state = AssistantState::Listening {
                    request_id: new_request_id,
                    start_time: std::time::Instant::now(),
                };

                outgoing_events.push(WsEvent::VadStart);
                info!("Assistant: Interrupted → Listening (new request_id: {})", new_request_id);
            }

            // Speaking → Idle on completion (placeholder)
            (AssistantState::Speaking { request_id }, WsEvent::VadEnd) => {
                self.current_state = AssistantState::Idle;

                if let Some(ctx) = self.requests.read().await.get(request_id) {
                    if let Some(latency) = ctx.latency_ms() {
                        info!("Request completed: {} (latency: {}ms)", request_id, latency);
                    }
                }

                // Clean up old requests (keep last 10)
                let mut requests = self.requests.write().await;
                if requests.len() > 10 {
                    let to_remove: Vec<_> = requests.keys().take(requests.len() - 10).cloned().collect();
                    for id in to_remove {
                        requests.remove(&id);
                    }
                }
            }

            // Pass through STT events
            (_, WsEvent::SttPartial { text } | WsEvent::SttFinal { text }) => {
                outgoing_events.push(event.clone());
            }

            // Ignore other events
            _ => {}
        }

        outgoing_events
    }

    pub fn current_state(&self) -> &AssistantState {
        &self.current_state
    }

    pub async fn get_request_context(&self, request_id: &Uuid) -> Option<RequestContext> {
        self.requests.read().await.get(request_id).cloned()
    }
}
