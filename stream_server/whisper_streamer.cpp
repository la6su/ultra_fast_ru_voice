#include "whisper_streamer.h"
#include <cstring>

static inline float pcm16_to_float(int16_t s) {
    return s / 32768.0f;
}

WhisperStreamer::WhisperStreamer(const std::string& model_path) {
    whisper_context_params cparams = whisper_context_default_params();
    cparams.use_gpu = true;

    ctx_ = whisper_init_from_file_with_params(model_path.c_str(), cparams);
}

WhisperStreamer::~WhisperStreamer() {
    if (ctx_) whisper_free(ctx_);
}

void WhisperStreamer::reset() {
    // Clear batch state for stateless operation
    batch_audio_.clear();
}

// ARCHITECTURE: Batch methods for stateless Rust integration
void WhisperStreamer::push_audio_batch(const float* samples, size_t count) {
    batch_audio_.assign(samples, samples + count);
}

std::vector<WhisperResult> WhisperStreamer::process_batch(bool final_result) {
    std::vector<WhisperResult> out;

    if (batch_audio_.empty()) {
        return out;
    }

    // CRITICAL: Ensure stateless operation between requests
    // params.no_context = true prevents context reuse
    // No additional reset needed with no_context=true

    whisper_full_params params =
        whisper_full_default_params(WHISPER_SAMPLING_GREEDY);

    // Latency-first settings per architecture requirements
    params.language = "ru";
    params.no_context = true;    // No context reuse between requests
    params.single_segment = true;
    params.print_progress = false;
    params.print_timestamps = false;

    if (whisper_full(ctx_, params, batch_audio_.data(), batch_audio_.size()) != 0) {
        batch_audio_.clear();
        return out;
    }

    int n = whisper_full_n_segments(ctx_);
    for (int i = 0; i < n; ++i) {
        const char* txt = whisper_full_get_segment_text(ctx_, i);
        if (txt && std::strlen(txt)) {
            out.push_back({txt, final_result});
        }
    }

    // Clear batch state
    batch_audio_.clear();

    return out;
}
