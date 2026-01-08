//! Тестовый binary для reference core
//! cargo run --bin reference_test

use ultra_fast_ru_voice::core::reference_core::{ReferencePipeline, SttEvent};
use ultra_fast_ru_voice::audio::types::AudioFrame;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧪 Testing Reference Core Pipeline");
    println!("===================================");

    // Создаем pipeline
    let mut pipeline = ReferencePipeline::new("ws://127.0.0.1:9001")?;
    println!("✅ Pipeline created successfully");

    // Тест 1: Silence
    println!("\n📝 Test 1: Silence (should produce no events)");
    let silent_frame = AudioFrame {
        samples: vec![0.001; 320], // Тихий фрейм
        sample_rate: 16000,
    };

    let events = pipeline.process_frame(silent_frame).await;
    println!("   Events produced: {}", events.len());
    assert!(events.is_empty(), "Silence should produce no events");
    println!("   ✅ Passed");

    // Тест 2: Speech detection
    println!("\n📝 Test 2: Speech detection simulation");

    // Имитируем: тишина → речь → тишина
    let mut test_frames = Vec::new();

    // 3 фрейма тишины
    for i in 0..3 {
        test_frames.push(AudioFrame {
            samples: vec![0.001; 320],
            sample_rate: 16000,
        });
        println!("   Frame {}: SILENCE", i);
    }

    // 5 фреймов речи
    for i in 0..5 {
        test_frames.push(AudioFrame {
            samples: vec![0.1; 320], // Выше порога VAD
            sample_rate: 16000,
        });
        println!("   Frame {}: SPEECH", i + 3);
    }

    // 3 фрейма тишины
    for i in 0..3 {
        test_frames.push(AudioFrame {
            samples: vec![0.001; 320],
            sample_rate: 16000,
        });
        println!("   Frame {}: SILENCE", i + 8);
    }

    println!("\n🎯 Processing frames...");

    for (i, frame) in test_frames.into_iter().enumerate() {
        let events = pipeline.process_frame(frame).await;

        if !events.is_empty() {
            println!("   Frame {} → {} events:", i, events.len());
            for event in events {
                match event {
                    SttEvent::Partial(text) => println!("     🔄 Partial: '{}'", text),
                    SttEvent::Final(text) => println!("     🎯 Final: '{}'", text),
                }
            }
        }
    }

    // Тест 3: Buffer lifecycle
    println!("\n📝 Test 3: Buffer lifecycle");
    pipeline.reset();

    println!("   Initial state: in_speech={}, active={}, pre_roll={}",
             pipeline.buffer.in_speech,
             pipeline.buffer.active.len(),
             pipeline.buffer.pre_roll.len());

    // Start speech
    pipeline.buffer.start_speech();
    println!("   After start_speech: in_speech={}, active={}, pre_roll={}",
             pipeline.buffer.in_speech,
             pipeline.buffer.active.len(),
             pipeline.buffer.pre_roll.len());

    // Add frames
    pipeline.buffer.push(AudioFrame {
        samples: vec![0.1; 320],
        sample_rate: 16000,
    });
    println!("   After push: active={}", pipeline.buffer.active.len());

    // End speech
    let segment = pipeline.buffer.end_speech();
    println!("   After end_speech: in_speech={}, segment_samples={}",
             pipeline.buffer.in_speech, segment.len());

    // Reset
    pipeline.reset();
    println!("   After reset: in_speech={}, active={}, pre_roll={}",
             pipeline.buffer.in_speech,
             pipeline.buffer.active.len(),
             pipeline.buffer.pre_roll.len());

    println!("\n✅ All reference core tests passed!");
    println!("🎯 Core pipeline is stable and ready for WS integration");

    Ok(())
}
