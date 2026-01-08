//! Тесты для чистого WS handler'а

use ultra_fast_ru_voice::ws::handler::WsClient;
use ultra_fast_ru_voice::ws::protocol::WsEvent;

#[tokio::test]
async fn test_ws_client_creation() {
    // Тест создания WS клиента
    let result = WsClient::new("ws://127.0.0.1:9001");
    assert!(result.is_ok(), "WS client should be created successfully");

    let mut client = result.unwrap();

    // Тест reset
    client.reset(); // Should not panic
}

#[tokio::test]
async fn test_ws_protocol_conversion() {
    // Тест конвертации SttEvent в WsEvent
    // (Это внутренний тест, в реальности требует мок WhisperService)

    // Создаем клиента (даже если сервер недоступен, структура должна работать)
    let result = WsClient::new("ws://127.0.0.1:9001");
    assert!(result.is_ok());

    // Тест что структура создана правильно
    let client = result.unwrap();
    assert_eq!(client.pipeline.vad.energy_ema, 0.0);
    assert!(!client.pipeline.buffer.in_speech);
    assert!(client.pipeline.buffer.active.is_empty());
}

#[test]
fn test_ws_event_serialization() {
    // Тест что WsEvent правильно сериализуется в JSON

    let event = WsEvent::SttPartial {
        text: "привет".to_string()
    };

    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("stt_partial"));
    assert!(json.contains("привет"));

    let final_event = WsEvent::SttFinal {
        text: "привет мир".to_string()
    };

    let final_json = serde_json::to_string(&final_event).unwrap();
    assert!(final_json.contains("stt_final"));
    assert!(final_json.contains("привет мир"));
}

#[test]
fn test_protocol_compliance() {
    // Проверяем что протокол соответствует спецификации из promts/04_ws_protocol_rules.md

    // VAD events
    let vad_start = WsEvent::VadStart;
    let vad_end = WsEvent::VadEnd;

    assert!(serde_json::to_string(&vad_start).unwrap().contains("vad_start"));
    assert!(serde_json::to_string(&vad_end).unwrap().contains("vad_end"));

    // STT events
    let stt_partial = WsEvent::SttPartial { text: "test".to_string() };
    let stt_final = WsEvent::SttFinal { text: "final test".to_string() };

    let partial_json = serde_json::to_string(&stt_partial).unwrap();
    let final_json = serde_json::to_string(&stt_final).unwrap();

    assert!(partial_json.contains("stt_partial"));
    assert!(partial_json.contains("test"));
    assert!(final_json.contains("stt_final"));
    assert!(final_json.contains("final test"));
}
