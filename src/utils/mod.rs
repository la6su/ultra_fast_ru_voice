//! # Utils - Утилиты для оптимизаций производительности
//!
//! Этот модуль содержит различные оптимизации для production использования:
//! - Memory pooling для снижения аллокаций
//! - GPU detection для автоматической конфигурации
//! - Latency tracking для мониторинга производительности
//! - Health checks для production monitoring
//!
//! ## Основные компоненты:
//! - **memory_pool**: Переиспользование AudioFrame объектов
//! - **gpu**: Авто-детект GPU и оптимизация threads
//! - **latency**: Статистический анализ задержек
//! - **health**: Мониторинг состояния системы

use std::sync::Arc;
use tracing::info;

/// Memory pool для AudioFrame'ов
pub mod memory_pool {
    use std::sync::Mutex;
    use super::*;

    #[derive(Debug, Clone)]
    pub struct AudioFramePool {
        pool: Arc<Mutex<Vec<crate::audio::types::AudioFrame>>>,
        max_size: usize,
    }

    impl AudioFramePool {
        pub fn new(max_size: usize) -> Self {
            Self {
                pool: Arc::new(Mutex::new(Vec::with_capacity(max_size))),
                max_size,
            }
        }

        pub fn get(&self) -> crate::audio::types::AudioFrame {
            let mut pool = self.pool.lock().unwrap();
            pool.pop().unwrap_or_else(|| crate::audio::types::AudioFrame {
                samples: Vec::with_capacity(320), // 20ms @ 16kHz
                sample_rate: 16000,
            })
        }

        pub fn put(&self, frame: crate::audio::types::AudioFrame) {
            let mut pool = self.pool.lock().unwrap();
            if pool.len() < self.max_size {
                // Очищаем samples для reuse
                let mut cleared_frame = frame;
                cleared_frame.samples.clear();
                pool.push(cleared_frame);
            }
        }
    }
}

/// GPU detection и CUDA configuration
pub mod gpu {
    use std::process::Command;
    use tracing::{info, warn};

    #[derive(Debug, Clone)]
    pub struct GpuInfo {
        pub has_cuda: bool,
        pub cuda_version: Option<String>,
        pub gpu_name: Option<String>,
        pub gpu_memory: Option<String>,
    }

    impl GpuInfo {
        pub fn detect() -> Self {
            let mut info = GpuInfo {
                has_cuda: false,
                cuda_version: None,
                gpu_name: None,
                gpu_memory: None,
            };

            // Проверяем nvidia-smi
            if let Ok(output) = Command::new("nvidia-smi")
                .arg("--query-gpu=name,memory.total")
                .arg("--format=csv,noheader,nounits")
                .output()
            {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let lines: Vec<&str> = stdout.lines().collect();

                    if let Some(first_line) = lines.first() {
                        let parts: Vec<&str> = first_line.split(',').map(|s| s.trim()).collect();
                        if parts.len() >= 2 {
                            info.has_cuda = true;
                            info.gpu_name = Some(parts[0].to_string());
                            info.gpu_memory = Some(format!("{} MB", parts[1]));
                        }
                    }
                }
            }

            // Проверяем CUDA runtime
            if let Ok(output) = Command::new("nvcc").arg("--version").output() {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        if line.contains("release") {
                            info.cuda_version = Some(line.trim().to_string());
                            break;
                        }
                    }
                }
            }

            if info.has_cuda {
                info!("🎮 GPU detected: {} ({})",
                      info.gpu_name.as_deref().unwrap_or("Unknown"),
                      info.gpu_memory.as_deref().unwrap_or("Unknown"));
                if let Some(version) = &info.cuda_version {
                    info!("🟢 CUDA version: {}", version);
                }
            } else {
                warn!("⚠️  No CUDA GPU detected, using CPU mode");
            }

            info
        }
    }

    pub fn optimize_whisper_threads(gpu_info: &GpuInfo) -> i32 {
        if gpu_info.has_cuda {
            // С GPU используем меньше CPU threads
            2
        } else {
            // CPU-only: используем все доступные cores
            num_cpus::get_physical() as i32
        }
    }
}

/// Latency monitoring и benchmarks
pub mod latency {
    use std::time::{Duration, Instant};
    use std::collections::VecDeque;
    use tracing::info;

    #[derive(Debug)]
    pub struct LatencyTracker {
        measurements: VecDeque<Duration>,
        max_samples: usize,
    }

    impl LatencyTracker {
        pub fn new(max_samples: usize) -> Self {
            Self {
                measurements: VecDeque::with_capacity(max_samples),
                max_samples,
            }
        }

        pub fn record(&mut self, duration: Duration) {
            self.measurements.push_back(duration);
            if self.measurements.len() > self.max_samples {
                self.measurements.pop_front();
            }
        }

        pub fn average(&self) -> Option<Duration> {
            if self.measurements.is_empty() {
                return None;
            }

            let total: Duration = self.measurements.iter().sum();
            Some(total / self.measurements.len() as u32)
        }

        pub fn percentile(&self, p: f64) -> Option<Duration> {
            if self.measurements.is_empty() {
                return None;
            }

            let mut sorted: Vec<Duration> = self.measurements.iter().cloned().collect();
            sorted.sort();

            let index = ((sorted.len() - 1) as f64 * p / 100.0) as usize;
            Some(sorted[index])
        }

        pub fn report(&self) {
            if let Some(avg) = self.average() {
                if let Some(p95) = self.percentile(95.0) {
                    if let Some(p99) = self.percentile(99.0) {
                        info!("📊 Latency: avg={}ms, p95={}ms, p99={}ms, samples={}",
                              avg.as_millis(), p95.as_millis(), p99.as_millis(), self.measurements.len());
                    }
                }
            }
        }
    }

    /// Scoped timer для измерения latency
    pub struct LatencyTimer<'a> {
        tracker: Option<&'a mut LatencyTracker>,
        start: Instant,
    }

    impl<'a> LatencyTimer<'a> {
        pub fn new(tracker: Option<&'a mut LatencyTracker>) -> Self {
            Self {
                tracker,
                start: Instant::now(),
            }
        }
    }

    impl<'a> Drop for LatencyTimer<'a> {
        fn drop(&mut self) {
            if let Some(tracker) = self.tracker.as_mut() {
                tracker.record(self.start.elapsed());
            }
        }
    }
}

/// Health checks и monitoring
pub mod health {
    use std::time::{Duration, Instant};
    use tracing::{info, warn, error};

    #[derive(Debug)]
    pub struct HealthChecker {
        last_check: Instant,
        check_interval: Duration,
        is_healthy: bool,
    }

    impl HealthChecker {
        pub fn new(check_interval: Duration) -> Self {
            Self {
                last_check: Instant::now(),
                check_interval,
                is_healthy: true,
            }
        }

        pub async fn check(&mut self) -> bool {
            if self.last_check.elapsed() < self.check_interval {
                return self.is_healthy;
            }

            self.last_check = Instant::now();

            // Проверяем основные компоненты
            let whisper_ok = check_whisper_server().await;
            let system_ok = check_system_resources();

            self.is_healthy = whisper_ok && system_ok;

            if !self.is_healthy {
                error!("🚨 Health check failed: whisper={}, system={}", whisper_ok, system_ok);
            }

            self.is_healthy
        }
    }

    async fn check_whisper_server() -> bool {
        // Проверяем подключение к C++ whisper серверу
        match tokio::net::TcpStream::connect("127.0.0.1:9001").await {
            Ok(_) => true,
            Err(_) => {
                warn!("❌ Whisper server not responding");
                false
            }
        }
    }

    fn check_system_resources() -> bool {
        // Проверяем основные системные ресурсы
        // В production можно добавить проверки memory, CPU, disk space
        true
    }
}
