```json
{
  "audio_frame": {
    "sample_rate": 16000,
    "channels": 1,
    "format": "pcm_f32",
    "frame_ms": 20,
    "normalization": "-1.0..1.0"
  },
  "vad_state": ["silence", "speech", "end_of_speech"],
  "vad_transitions": {
    "silence": ["speech"],
    "speech": ["speech", "end_of_speech"],
    "end_of_speech": ["silence"]
  },
  "vad_constraints": {
    "no_audio_accumulation": true,
    "no_stt_calls": true,
    "no_transport_knowledge": true,
    "non_blocking": true
  }
}
```

## VAD Algorithm Parameters
- `start_multiplier`: 3.0 (noise_floor × start_multiplier)
- `end_multiplier`: 1.5 (noise_floor × end_multiplier)
- `hangover_frames`: 8 (≈160ms cooldown)
- `ema_alpha`: 0.1 (energy smoothing)
