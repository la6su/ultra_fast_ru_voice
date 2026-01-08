//! Тесты для reference minimal core
//! Проверяем фиксированный контракт без WebSocket

use ultra_fast_ru_voice::core::reference_core::{ReferencePipeline, SttEvent};
use ultra_fast_ru_voice::audio::types::AudioFrame;

#[tokio::test]
async fn test_silence_no_events() {
    let mut pipeline = ReferencePipeline::new("ws://127.0.0.1:9001")
        .expect("Failed to create pipeline");

    // Тихий фрейм (ниже порога VAD)
    let silent_frame = AudioFrame {
        samples: vec![0.001; 320], // 20ms @ 16kHz
        sample_rate: 16000,
    };

    let events = pipeline.process_frame(silent_frame).await;
    assert!(events.is_empty(), "Silence should produce no events");
}

#[tokio::test]
async fn test_speech_detection() {
    let mut pipeline = ReferencePipeline::new("ws://127.0.0.1:9001")
        .expect("Failed to create pipeline");

    // Создаем тестовый сигнал: тишина → речь → тишина
    let mut frames = Vec::new();

    // 5 фреймов тишины (ниже порога)
    for _ in 0..5 {
        frames.push(AudioFrame {
            samples: vec![0.001; 320],
            sample_rate: 16000,
        });
    }

    // 10 фреймов речи (выше порога)
    for _ in 0..10 {
        frames.push(AudioFrame {
            samples: vec![0.1; 320], // Выше порога VAD
            sample_rate: 16000,
        });
    }

    // 5 фреймов тишины
    for _ in 0..5 {
        frames.push(AudioFrame {
            samples: vec![0.001; 320],
            sample_rate: 16000,
        });
    }

    let mut speech_started = false;
    let mut speech_ended = false;

    // Обрабатываем фреймы (тест может fall если C++ сервер не запущен)
    for frame in frames {
        let events = pipeline.process_frame(frame).await;

        for event in events {
            match event {
                SttEvent::Partial(text) => {
                    println!("Partial: {}", text);
                    assert!(speech_started, "Partial should come after speech start");
                }
                SttEvent::Final(text) => {
                    println!("Final: {}", text);
                    speech_ended = true;
                }
            }
        }
    }

    // Проверяем что pipeline правильно обработал последовательность
    // (В реальном тесте нужно мокать WhisperService)
    pipeline.reset();
}

#[test]
fn test_pipeline_creation() {
    let result = ReferencePipeline::new("ws://127.0.0.1:9001");
    assert!(result.is_ok(), "Pipeline should be created successfully");
}

#[test]
fn test_buffer_lifecycle() {
    let mut pipeline = ReferencePipeline::new("ws://127.0.0.1:9001")
        .expect("Failed to create pipeline");

    // Проверяем начальное состояние
    assert!(!pipeline.buffer.in_speech);
    assert!(pipeline.buffer.active.is_empty());
    assert!(pipeline.buffer.pre_roll.is_empty());

    // Start speech
    pipeline.buffer.start_speech();
    assert!(pipeline.buffer.in_speech);
    assert!(pipeline.buffer.active.is_empty()); // pre-roll пустой

    // Add frames
    let frame = AudioFrame {
        samples: vec![0.1; 320],
        sample_rate: 16000,
    };
    pipeline.buffer.push(frame.clone());
    assert_eq!(pipeline.buffer.active.len(), 1);

    // End speech
    let segment = pipeline.buffer.end_speech();
    assert!(!pipeline.buffer.in_speech);
    assert!(pipeline.buffer.active.is_empty());
    assert!(!segment.is_empty());

    // Reset
    pipeline.reset();
    assert!(!pipeline.buffer.in_speech);
    assert!(pipeline.buffer.active.is_empty());
    assert!(pipeline.buffer.pre_roll.is_empty());
}
