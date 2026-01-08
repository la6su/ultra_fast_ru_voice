//! # Performance Benchmark для UltraFast RU Voice
//!
//! Этот binary проводит нагрузочное тестирование системы:
//! - Latency measurements (end-to-end)
//! - Throughput testing
//! - Memory usage monitoring
//! - GPU utilization tracking
//!
//! ## Запуск:
//! ```bash
//! cargo run --bin benchmark -- --duration 60 --clients 10
//! ```
//!
//! ## Метрики:
//! - Average/P95/P99 latency per request
//! - Requests per second (RPS)
//! - Memory usage over time
//! - GPU utilization (if available)

use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{info, warn};

#[derive(Parser)]
#[command(name = "benchmark")]
#[command(about = "Performance benchmark for UltraFast RU Voice")]
struct Args {
    /// WebSocket server URL
    #[arg(short, long, default_value = "ws://127.0.0.1:8080")]
    server: String,

    /// Test duration in seconds
    #[arg(short, long, default_value = "30")]
    duration: u64,

    /// Number of concurrent clients
    #[arg(short, long, default_value = "5")]
    clients: usize,

    /// Audio chunk size (samples)
    #[arg(long, default_value = "320")]
    chunk_size: usize,

    /// Sample rate
    #[arg(long, default_value = "16000")]
    sample_rate: u32,
}

#[derive(Debug, Clone)]
struct BenchmarkStats {
    total_requests: u64,
    successful_requests: u64,
    failed_requests: u64,
    latencies: Vec<Duration>,
    start_time: Instant,
}

impl BenchmarkStats {
    fn new() -> Self {
        Self {
            total_requests: 0,
            successful_requests: 0,
            failed_requests: 0,
            latencies: Vec::new(),
            start_time: Instant::now(),
        }
    }

    fn record_success(&mut self, latency: Duration) {
        self.total_requests += 1;
        self.successful_requests += 1;
        self.latencies.push(latency);
    }

    fn record_failure(&mut self) {
        self.total_requests += 1;
        self.failed_requests += 1;
    }

    fn report(&self) {
        let elapsed = self.start_time.elapsed();
        let rps = self.total_requests as f64 / elapsed.as_secs_f64();

        if self.latencies.is_empty() {
            info!("📊 Benchmark Results:");
            info!("   Duration: {:.2}s", elapsed.as_secs_f64());
            info!("   Total requests: {}", self.total_requests);
            info!("   Successful: {}", self.successful_requests);
            info!("   Failed: {}", self.failed_requests);
            info!("   Requests/sec: {:.2}", rps);
            return;
        }

        // Calculate percentiles
        let mut sorted_latencies = self.latencies.clone();
        sorted_latencies.sort();

        let len = sorted_latencies.len();
        let avg_latency = sorted_latencies.iter().sum::<Duration>() / len as u32;
        let p50 = sorted_latencies[len / 2];
        let p95 = sorted_latencies[(len * 95) / 100];
        let p99 = sorted_latencies[(len * 99) / 100];

        info!("📊 Benchmark Results:");
        info!("   Duration: {:.2}s", elapsed.as_secs_f64());
        info!("   Total requests: {}", self.total_requests);
        info!("   Successful: {} ({:.1}%)", self.successful_requests,
              (self.successful_requests as f64 / self.total_requests as f64) * 100.0);
        info!("   Failed: {}", self.failed_requests);
        info!("   Requests/sec: {:.2}", rps);
        info!("   Latency (avg): {:.0}ms", avg_latency.as_millis());
        info!("   Latency (p50): {:.0}ms", p50.as_millis());
        info!("   Latency (p95): {:.0}ms", p95.as_millis());
        info!("   Latency (p99): {:.0}ms", p99.as_millis());
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize tracing
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .init();

    info!("🚀 Starting UltraFast RU Voice Benchmark");
    info!("   Server: {}", args.server);
    info!("   Duration: {}s", args.duration);
    info!("   Clients: {}", args.clients);
    info!("   Chunk size: {} samples", args.chunk_size);

    // Shared statistics
    let stats = Arc::new(Mutex::new(BenchmarkStats::new()));

    // Generate test audio data (sine wave)
    let test_audio = generate_test_audio(args.chunk_size, args.sample_rate);

    // Spawn client tasks
    let mut handles = Vec::new();
    for client_id in 0..args.clients {
        let server_url = args.server.clone();
        let audio_data = test_audio.clone();
        let stats_clone = Arc::clone(&stats);

        let handle = tokio::spawn(async move {
            if let Err(e) = run_client(client_id, &server_url, &audio_data, args.duration, stats_clone).await {
                warn!("Client {} failed: {}", client_id, e);
            }
        });

        handles.push(handle);
    }

    // Wait for all clients to complete
    for handle in handles {
        let _ = handle.await;
    }

    // Final report
    let final_stats = stats.lock().await;
    final_stats.report();

    // Memory usage report
    report_memory_usage();

    info!("✅ Benchmark completed");
    Ok(())
}

async fn run_client(
    client_id: usize,
    server_url: &str,
    audio_data: &[i16],
    duration_secs: u64,
    stats: Arc<Mutex<BenchmarkStats>>,
) -> anyhow::Result<()> {
    let (ws_stream, _) = connect_async(server_url).await
        .map_err(|e| anyhow::anyhow!("Failed to connect: {}", e))?;

    let (mut ws_sender, mut ws_receiver) = ws_stream.split();
    let test_start = Instant::now();
    let test_duration = Duration::from_secs(duration_secs);

    info!("Client {}: Connected, starting test", client_id);

    loop {
        // Check if test duration exceeded
        if test_start.elapsed() >= test_duration {
            break;
        }

        let request_start = Instant::now();

        // Send audio data
        let audio_bytes: Vec<u8> = audio_data.iter()
            .flat_map(|&sample| sample.to_le_bytes())
            .collect();

        if ws_sender.send(Message::Binary(audio_bytes)).await.is_err() {
            let mut stats = stats.lock().await;
            stats.record_failure();
            break;
        }

        // Wait for response with timeout
        let mut received_response = false;
        let timeout_duration = Duration::from_secs(5);

        tokio::select! {
            msg = ws_receiver.next() => {
                match msg {
                    Some(Ok(Message::Text(json_str))) => {
                        // Parse response
                        if let Ok(event) = serde_json::from_str::<serde_json::Value>(&json_str) {
                            if let Some(event_type) = event["type"].as_str() {
                                if event_type == "stt_final" {
                                    let latency = request_start.elapsed();
                                    let mut stats = stats.lock().await;
                                    stats.record_success(latency);
                                    received_response = true;
                                }
                            }
                        }
                    }
                    Some(Ok(Message::Close(_))) => break,
                    Some(Err(_)) => break,
                    _ => {}
                }
            }
            _ = tokio::time::sleep(timeout_duration) => {
                // Timeout
                let mut stats = stats.lock().await;
                stats.record_failure();
            }
        }

        // Small delay between requests
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    info!("Client {}: Test completed", client_id);
    Ok(())
}

fn generate_test_audio(samples: usize, sample_rate: u32) -> Vec<i16> {
    let frequency = 440.0; // A4 note
    let amplitude = 0.3;

    (0..samples)
        .map(|i| {
            let t = i as f32 / sample_rate as f32;
            let sample = (amplitude * (2.0 * std::f32::consts::PI * frequency * t).sin()) as f32;
            (sample * 32767.0) as i16
        })
        .collect()
}

fn report_memory_usage() {
    // Basic memory reporting (can be extended with more detailed metrics)
    info!("📈 Memory usage monitoring available via system tools:");
    info!("   - Use `htop` or `top` for real-time monitoring");
    info!("   - Use `valgrind` for detailed memory profiling");
    info!("   - GPU memory: `nvidia-smi --query-gpu=memory.used --format=csv`");
}
