#pragma once
#include <string>
#include <vector>
#include <cstdint>
#include "whisper.h"

struct WhisperResult {
    std::string text;
    bool is_final;
};

class WhisperStreamer {
public:
    explicit WhisperStreamer(const std::string& model_path);
    ~WhisperStreamer();

    // ARCHITECTURE: Batch-only methods for stateless Rust integration
    // Один запрос = один сегмент = один результат
    void push_audio_batch(const float* samples, size_t count);
    std::vector<WhisperResult> process_batch(bool final_result);

    void reset();

private:
    whisper_context* ctx_ = nullptr;

    // Batch mode state (stateless between requests)
    std::vector<float> batch_audio_;
};
