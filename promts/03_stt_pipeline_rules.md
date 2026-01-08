stt_pipeline:
  orchestrator: "SttPipeline"
  responsibilities:
    - vad_events
    - audio_buffering
    - whisper_calls

pipeline_stages:
  - stage: vad
    input: audio_frame
    output: vad_event
    blocking: false
  - stage: buffer
    input: vad_event + audio_frame
    output: speech_segment
    blocking: false
  - stage: whisper
    input: speech_segment
    output: stt_event
    blocking: true

reference_behavior:
  speech_start: ["clear_buffer"]
  speech_frame: ["accumulate_samples", "optional_partial_decode"]
  speech_end: ["final_decode", "emit_final", "clear_buffer"]

constraints:
  - partial_decode: false  # not in reference core
  - overlap_decode: false  # not in reference core
  - streaming_decode: false # not in reference core
