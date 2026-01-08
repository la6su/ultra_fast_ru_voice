Extension policy:```yaml
extension_policy:
  rules:
    1: "Extension MUST NOT mutate core state"
    2: "Extension MAY observe events"
    3: "Extension MUST declare capabilities"

  allowed:
    - event_observation
    - additional_processing
    - custom_ui_components
    - alternative_transports

  forbidden:
    - core_state_mutation
    - pipeline_interruption
    - protocol_violation
    - performance_regression
```

## Extension Points

### Audio Pipeline Extensions
- Custom VAD algorithms
- Additional audio preprocessing
- Alternative STT backends

### UI Extensions
- Custom WebUI components
- Alternative client implementations
- Mobile app integrations

### LLM Extensions
- Custom model backends
- Alternative prompt strategies
- Specialized conversation flows

### TTS Extensions
- Alternative voice synthesis
- Custom pronunciation rules
- Multi-language support

## Quality Gates

All extensions must pass:
- Performance regression tests
- Compatibility validation
- Security audit
- Documentation review

## Implementation Guidelines

- Extensions are opt-in features
- Core functionality remains stable
- Breaking changes require new version
- Extensions may be deprecated but never removed without notice

## Feature Development Order

All new features MUST be added incrementally following this priority order:

### Phase 1: Core Enhancements (Completed)
1. ✅ **Partial STT** (streaming transcription during speech)
2. ✅ **Async Processing** (Tokio runtime, non-blocking operations)
3. ✅ **WebSocket Server** (binary audio input, JSON events output)

### Phase 2: Voice Assistant Features (In Progress)
4. 🔄 **LLM Integration** (streaming text generation)
5. 🔄 **TTS Integration** (Silero models, streaming audio output)
6. 🔄 **State Machine** (request-bound orchestration)

### Phase 3: Advanced Features (Future)
7. ⏳ **Overlap/Windowing** (sliding window for continuous transcription)
8. ⏳ **Streaming Whisper** (incremental decode, real-time hypotheses)
9. ⏳ **Latency Tuning** (GPU optimization, memory pooling, zero-copy)
10. ⏳ **WebUI Protocol Extensions** (full duplex audio, control messages)

## Development Rules

### Hard Rules
- **Reference core MUST always compile** (STT-only mode)
- **No breaking changes** without explicit "reference version" bump
- **All features optional** via feature flags or config
- **Backward compatibility** maintained

### Best Practices
- **Incremental changes** - small, testable commits
- **Performance measurement** after each change
- **Documentation updates** with code changes
- **Error handling** from day one

### Feature Flags (Proposed)

```toml
[features]
default = ["stt"]
stt = ["whisper"]
partial-stt = ["stt"]
llm = ["stt", "tokio"]
tts = ["llm"]
full-assistant = ["llm", "tts", "state-machine"]
```

### Versioning
- **Major**: Breaking API changes
- **Minor**: New features (LLM, TTS)
- **Patch**: Bug fixes, optimizations

### Testing Strategy
- Unit tests for all components
- Integration tests for pipelines
- Latency benchmarks
- Load testing with multiple clients
