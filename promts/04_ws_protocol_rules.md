```json
{
  "transport": "websocket",
  "binary_messages": {
    "direction": "client_to_server",
    "format": "pcm_s16_le",
    "sample_rate": 16000,
    "channels": 1,
    "chunk_ms": 20
  },
  "text_messages": {
    "direction": "server_to_client",
    "format": "json",
    "encoding": "utf8",
    "schema": "strict_enum"
  },
  "message_schemas": {
    "vad_start": {"type": "vad_start"},
    "vad_end": {"type": "vad_end"},
    "stt_partial": {"type": "stt_partial", "text": "string"},
    "stt_final": {"type": "stt_final", "text": "string"}
  },
  "constraints": {
    "no_vad_operations": true,
    "no_stt_calls": true,
    "no_business_logic": true,
    "only_transport": true
  }
}
```
