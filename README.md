# UltraFast RU Voice Assistant

Гипер-быстрый голосовой ассистент на русском языке с latency <300ms end-to-end.

## Архитектура

```
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
```

## Быстрый запуск

### 1. Клонировать репозиторий
```bash
git clone <repository-url>
cd ultra_fast_ru_voice
```

### 2. Настроить C++ whisper сервер
```bash
# В LXC контейнере Ubuntu (уже настроено)
cd stream_server
cmake -B build
cmake --build build --config Release
./build/whisper_stream_server
```

### 3. Настроить переменные окружения
```bash
cp .env.example .env
# Отредактировать .env с API ключами
```

### 4. Запустить Rust сервер
```bash
cargo run
```

### 5. Протестировать
Открыть `test_websocket.html` в браузере и протестировать.

## Конфигурация

### Переменные окружения (.env)

```bash
# OpenAI API (для LLM)
OPENAI_API_KEY=sk-your-key-here

# Whisper C++ сервер
WHISPER_CPP_SERVER=127.0.0.1:9001

# Rust WebSocket сервер
WS_HOST=127.0.0.1
WS_PORT=8080

# TTS настройки
TTS_MODEL=silero

# Логирование
RUST_LOG=info
```

## API

### WebSocket протокол

#### Входящие сообщения (Client → Server)
- **Binary**: PCM16 audio frames (16kHz, mono, little-endian)

#### Исходящие сообщения (Server → Client)
```json
// VAD события
{"type": "vad_start"}
{"type": "vad_end"}

// STT события
{"type": "stt_partial", "text": "привет"}
{"type": "stt_final", "text": "привет как дела"}

// LLM события
{"type": "llm_partial", "text": "привет"}
{"type": "llm_final", "text": "привет, чем могу помочь?"}

// TTS события
{"type": "tts_start"}
{"type": "tts_audio", "data": "base64-encoded-pcm16"}
{"type": "tts_end"}
```

## Разработка

### Структура проекта
```
src/
├── main.rs              # Точка входа, WebSocket сервер
├── audio/               # Аудио обработка, VAD
├── core/                # State machine, STT pipeline
├── stt/                 # Whisper клиент
├── llm/                 # LLM клиент (OpenAI)
├── tts/                 # TTS клиент (Silero)
└── ws/                  # WebSocket протокол

stream_server/          # C++ whisper сервер
promts/                  # Документация и спецификации
```

### Сборка и тестирование
```bash
# Сборка
cargo build --release

# Тестирование
cargo test

# Запуск с логами
RUST_LOG=debug cargo run
```

### Performance цели
- **VAD**: <100ms detection
- **STT**: <120ms inference (Whisper)
- **LLM**: <150ms first token (OpenAI)
- **TTS**: <80ms first audio (Silero)
- **End-to-end**: <300ms total

## Production развертывание

### Docker
```dockerfile
FROM rust:1.75-slim as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates
COPY --from=builder /app/target/release/ultra_fast_ru_voice /usr/local/bin/
EXPOSE 8080
CMD ["ultra_fast_ru_voice"]
```

### Системные требования
- **CPU**: 4+ cores (для Whisper)
- **RAM**: 4GB+ (для моделей)
- **GPU**: NVIDIA с CUDA (рекомендуется)
- **OS**: Linux (Ubuntu 20.04+)

## Contributing

1. Fork репозиторий
2. Создать feature branch
3. Добавить тесты
4. Проверить performance impact
5. Создать PR

## License

MIT License - см. LICENSE файл.
