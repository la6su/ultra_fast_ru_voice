# Architecture Rules```yaml
architecture:
  layers:
    - audio_input
    - vad
    - stt_pipeline
    - ws_transport

  invariants:
    - single_source_of_truth: "SttPipeline orchestrates all VAD/buffer/Whisper operations"
    - no_business_logic_in_ws: "WebSocket layer only transports, no processing"
    - stateless_vad: "VAD emits events only, doesn't accumulate audio"
    - async_pipeline: "All operations are async and non-blocking"

  forbidden:
    - direct_audio_to_llm: "Audio must go through VAD → Buffer → Whisper pipeline"
    - ws_state_mutation: "WebSocket handlers cannot modify business state"
    - vad_audio_accumulation: "VAD cannot buffer or store audio data"
    - transport_business_logic: "Transport layer cannot make business decisions"

  contracts:
    - vad: "AudioFrame → VadEvent (SpeechStart/SpeechEnd)"
    - buffer: "VadEvent + AudioFrame → SpeechSegment"
    - whisper: "SpeechSegment → SttEvent (Partial/Final)"
    - ws: "Binary PCM16 → SttEvent → JSON WsEvent"
```

## Core Principles

1. **Async-First**: All I/O and compute operations are async using Tokio runtime
2. **Request-Bound**: Every user intent has unique `request_id`, all operations are scoped to it
3. **Decoupled Workers**: STT, LLM, TTS run as independent async tasks
4. **No Blocking**: Audio pipeline never blocks, all heavy operations spawn_blocking
5. **State Machine**: Explicit state transitions with logging and cancellation

## Component Separation

- **VAD**: Stateless energy detection, emits only SpeechStart/SpeechEnd
- **STT Pipeline**: Orchestrates VAD events, audio buffering, Whisper calls
- **WebSocket Handler**: Adapts I/O to pipeline calls, no business logic
- **State Machine**: Manages assistant lifecycle (Idle → Listening → Thinking → Speaking)

## Data Flow

```
Audio Input → VAD → STT Pipeline → State Machine → LLM/TTS → Audio Output
     ↓         ↓         ↓            ↓            ↓           ↓
   WS/Mic   Events   Events      Commands    Streaming   WS/Speaker
```

## Error Handling

- All errors recoverable, no panics
- Graceful degradation with voice fallbacks
- Structured logging with tracing
- Health checks and metrics

## Performance

- Target: ≤300ms end-to-end latency
- Memory pooling for frames
- Zero-copy where possible
- GPU acceleration for ML models
