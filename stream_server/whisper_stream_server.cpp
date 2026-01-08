//! # Whisper Batch Inference Server
//!
//! Чистый HTTP POST сервер для batch transcription.
//! Принимает готовые speech segments, возвращает финальный текст.
//!
//! ## Архитектура:
//! - Stateless: Каждый запрос независимый, no context reuse
//! - Batch-only: Один сегмент = один результат
//! - No VAD: Сегментация происходит в Rust orchestrator
//! - HTTP POST: /transcribe endpoint

#include <App.h>
#include "whisper_streamer.h"
#include <vector>
#include <string>
#include <iostream>

// ===============================
// Main - HTTP-only batch server
// ===============================
int main() {
    uWS::App()
        // ===========================
        // HTTP POST /transcribe - единственный endpoint
        // ===========================
        .post("/transcribe", [](auto* res, auto* req) {
            // ARCHITECTURE: Stateless batch inference worker
            // Input: PCM16 speech segment from Rust
            // Output: JSON with final transcription
            // No VAD, no streaming, no context - pure batch processing

            // Validate request
            auto contentLength = req->getHeader("content-length");
            if (contentLength.empty()) {
                res->writeStatus("400 Bad Request")
                    ->writeHeader("Content-Type", "application/json")
                    ->end("{\"error\": \"Missing Content-Length\"}");
                return;
            }

            // Get PCM16 audio data
            std::string_view body = req->getQuery();
            if (body.empty()) {
                res->writeStatus("400 Bad Request")
                    ->writeHeader("Content-Type", "application/json")
                    ->end("{\"error\": \"Empty request body\"}");
                return;
            }

            // Convert PCM16 → float32
            const int16_t* pcm = reinterpret_cast<const int16_t*>(body.data());
            size_t total_samples = body.size() / sizeof(int16_t);

            if (total_samples == 0) {
                res->writeStatus("400 Bad Request")
                    ->writeHeader("Content-Type", "application/json")
                    ->end("{\"error\": \"Empty audio data\"}");
                return;
            }

            std::vector<float> audio_float(total_samples);
            for (size_t i = 0; i < total_samples; ++i) {
                audio_float[i] = pcm[i] / 32768.0f;
            }

            // Initialize WhisperStreamer (singleton per process)
            static WhisperStreamer* whisper_instance = nullptr;
            if (!whisper_instance) {
                try {
                    whisper_instance = new WhisperStreamer("../models/ggml-small.bin");
                    std::cout << "🎯 Whisper model loaded successfully\n";
                } catch (const std::exception& e) {
                    res->writeStatus("500 Internal Server Error")
                        ->writeHeader("Content-Type", "application/json")
                        ->end("{\"error\": \"Failed to load Whisper model\"}");
                    return;
                }
            }

            // CRITICAL: Reset context between requests (stateless!)
            whisper_instance->reset();

            // Batch transcription
            std::string final_text;
            try {
                whisper_instance->push_audio_batch(audio_float.data(), audio_float.size());
                auto results = whisper_instance->process_batch(true);

                for (auto& r : results) {
                    final_text += r.text;
                }
            } catch (const std::exception& e) {
                res->writeStatus("500 Internal Server Error")
                    ->writeHeader("Content-Type", "application/json")
                    ->end("{\"error\": \"Transcription failed\"}");
                return;
            }

            // Return result
            std::string response = "{\"text\": \"" + final_text + "\"}";
            res->writeStatus("200 OK")
                ->writeHeader("Content-Type", "application/json")
                ->end(response);
        })

        // ===========================
        // Listen on port 9001
        // ===========================
        .listen(9001, [](auto* token) {
            if (token) {
                std::cout << "🟢 Whisper Batch Server listening on :9001 (HTTP POST /transcribe)\n";
            } else {
                std::cerr << "❌ Failed to listen on port 9001\n";
            }
        })

        .run();
}
