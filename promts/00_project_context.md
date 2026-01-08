You are working on a project called "ultra_fast_ru_voice".

This is a ultra-low-latency Russian voice assistant built around:
- Rust core (Tokio async runtime)
- Whisper ASR (whisper.cpp backend)
- Energy-based VAD
- WebSocket transport
- LLM integration (streaming)
- TTS integration (Silero models)
- State machine orchestration

The project has a stable "Reference Minimal Core" (STT-only) which MUST remain compilable.
Extensions are added as optional features following the extension policy.

Current architecture:
┌──────────────────────────────────────────────┐
│ Single Server (GPU)                           │
│                                              │
│ ┌──────────────┐                             │
│ │ Web UI       │  ← browser (WebSocket)      │
│ └──────┬───────┘                             │
│        │                                     │
│ ┌──────▼──────────────────────────────┐     │
│ │ Rust Core (orchestrator)             │     │
│ │                                      │     │
│ │ ├── Audio In (WS / mic)              │     │
│ │ ├── VAD (Rust)                       │     │
│ │ ├── STT Client                       │◄────┼── whisper.cpp
│ │ ├── LLM Streaming                    │     │
│ │ ├── TTS Client                       │◄────┼── Silero (Torch)
│ │ ├── State Machine                    │     │
│ │ └── WebSocket API                    │     │
│ └──────────────────────────────────────┘     │
│                                              │
│ ┌────────────────────┐   ┌────────────────┐ │
│ │ whisper.cpp        │   │ Silero TTS      │ │
│ │ (CUDA, C++)        │   │ (PyTorch, CUDA) │ │
│ └────────────────────┘   └────────────────┘ │
└──────────────────────────────────────────────┘

Target latency: ≤300ms end-to-end response.
All components are async and request-bound.
