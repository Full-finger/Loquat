//! Performance monitoring aspect implementation

use crate::aop::traits::Aspect;
use crate::errors::{AopError, Result};
use crate::logging::traits::{Logger, LogLevel, LogContext};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, Instant};

pub struct PerformanceAspect {
    logger: Arc<dyn Logger>,
    slow_threshold: Duration,
    track_memory: bool,
    track_cpu: bool,
    enable_metrics: bool,
}

impl PerformanceAspect {
    pub fn new(logger: Arc<dyn Logger>) -> Self {
        Self {
            logger,
            slow_threshold: Duration::from_millis(1000),
            track_memory: false,
            track_cpu: false,
            enable_metrics: true,
        }
    }

    pub fn with_slow_threshold(mut self, threshold: Duration) -> Self {
        self.slow_threshold = threshold;
        self
    }

    pub fn with_memory_tracking(mut self, track: bool) -> Self {
        self.track_memory = track;
        self
    }

    pub fn with_cpu_tracking(mut self, track: bool) -> Self {
        self.track_cpu = track;
        self
    }

    pub fn with_metrics(mut self, enable: bool) -> Self {
        self.enable_metrics = enable;
        self
    }

    pub fn web_request_monitor(logger: Arc<dyn Logger>) -> Self {
        Self {
            logger,
            slow_threshold: Duration::from_millis(500),
            track_memory: true,
            track_cpu: false,
            enable_metrics: true,
        }
    }

    pub fn database_monitor(logger: Arc<dyn Logger>) -> Self {
        Self {
            logger,
            slow_threshold: Duration::from_millis(200),
            track_memory: false,
            track_cpu: false,
            enable_metrics: true,
        }
    }

    pub fn background_job_monitor(logger: Arc<dyn Logger>) -> Self {
        Self {
            logger,
            slow_threshold: Duration::from_secs(30),
            track_memory: true,
            track_cpu: true,
            enable_metrics: true,
        }
    }

    pub fn get_memory_usage(&self) -> Option<(usize, usize)> {
        if !self.track_memory {
            return None;
        }

        match std::process::Command::new("ps").arg("-o").arg("vsz,rss").arg("-p").arg(&std::process::id().to_string()).output() {
            Ok(output) => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = output_str.lines().collect();
                if lines.len() >= 2 {
                    let parts: Vec<&str> = lines[1].split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let (Ok(vsz), Ok(rss)) = (parts[0].parse::<usize>(), parts[1].parse::<usize>()) {
                            return Some((vsz * 1024, rss * 1024));
                        }
                    }
                }
                None
            },
            Err(_) => None,
        }
    }

    pub fn get_cpu_usage(&self) -> Option<f64> {
        if !self.track_cpu {
            return None;
        }

        match std::process::Command::new("ps").arg("-o").arg("%cpu").arg("-p").arg(&std::process::id().to_string()).output() {
            Ok(output) => {
                let output_str = String::from_utf8_lossy(&output.stdout);
                let lines: Vec<&str> = output_str.lines().collect();
                if lines.len() >= 2 {
                    if let Ok(cpu_percent) = lines[1].trim().parse::<f64>() {
                        return Some(cpu_percent);
                    }
                }
                None
            },
            Err(_) => None,
        }
    }
}

#[async_trait]
impl Aspect for PerformanceAspect {
    async fn before(&self, operation: &str) -> Result<()> {
        if !self.enable_metrics {
            return Ok(());
        }

        let mut log_context = LogContext::new();
        log_context = log_context.with_metadata("operation", operation)?;
        log_context = log_context.with_metadata("phase", "start")?;

        if let Some((vsz, rss)) = self.get_memory_usage() {
            log_context = log_context.with_metadata("memory_vsz_initial", vsz)?;
            log_context = log_context.with_metadata("memory_rss_initial", rss)?;
        }

        if let Some(cpu) = self.get_cpu_usage() {
            log_context = log_context.with_metadata("cpu_initial", cpu)?;
        }

        self.logger.log_with_context(
            LogLevel::Debug,
            &format!("Starting performance monitoring for {}", operation),
            &log_context,
        );

        Ok(())
    }

    async fn after(&self, operation: &str, result: &Result<()>) -> Result<()> {
        if !self.enable_metrics {
            return Ok(());
        }

        let mut log_context = LogContext::new();
        log_context = log_context.with_metadata("operation", operation)?;
        log_context = log_context.with_metadata("success", result.is_ok())?;

        if let Some((vsz, rss)) = self.get_memory_usage() {
            log_context = log_context.with_metadata("memory_vsz_final", vsz)?;
            log_context = log_context.with_metadata("memory_rss_final", rss)?;
        }

        if let Some(cpu) = self.get_cpu_usage() {
            log_context = log_context.with_metadata("cpu_final", cpu)?;
        }

        let log_level = if result.is_err() {
            LogLevel::Error
        } else {
            LogLevel::Info
        };

        self.logger.log_with_context(
            log_level,
            &format!("Completed performance monitoring for {}", operation),
            &log_context,
        );

        Ok(())
    }

    async fn on_error(&self, operation: &str, error: &AopError) -> Result<()> {
        let mut log_context = LogContext::new();
        log_context = log_context.with_metadata("operation", operation)?;
        log_context = log_context.with_metadata("error_type", std::any::type_name_of_val(error))?;
        log_context = log_context.with_metadata("error_message", error.to_string())?;

        self.logger.log_with_context(
            LogLevel::Error,
            &format!("Performance monitoring error in {}", operation),
            &log_context,
        );

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PerformanceMetrics {
    pub operation: String,
    pub start_time: Instant,
    pub end_time: Option<Instant>,
    pub success: Option<bool>,
    pub memory_initial: Option<(usize, usize)>,
    pub memory_final: Option<(usize, usize)>,
    pub cpu_initial: Option<f64>,
    pub cpu_final: Option<f64>,
}

impl PerformanceMetrics {
    pub fn new(operation: &str) -> Self {
        Self {
            operation: operation.to_string(),
            start_time: Instant::now(),
            end_time: None,
            success: None,
            memory_initial: None,
            memory_final: None,
            cpu_initial: None,
            cpu_final: None,
        }
    }

    pub fn complete(&mut self, success: bool) {
        self.end_time = Some(Instant::now());
        self.success = Some(success);
    }

    pub fn duration(&self) -> Option<Duration> {
        self.end_time.map(|end| end.duration_since(self.start_time))
    }

    pub fn memory_delta(&self) -> Option<(isize, isize)> {
        match (self.memory_initial, self.memory_final) {
            (Some(initial), Some(final_res)) => Some((
                final_res.0 as isize - initial.0 as isize,
                final_res.1 as isize - initial.1 as isize,
            )),
            _ => None,
        }
    }

    pub fn cpu_delta(&self) -> Option<f64> {
        match (self.cpu_initial, self.cpu_final) {
            (Some(initial), Some(final_res)) => Some(final_res - initial),
            _ => None,
        }
    }
}

pub struct PerformanceAspectBuilder {
    logger: Option<Arc<dyn Logger>>,
    slow_threshold: Duration,
    track_memory: bool,
    track_cpu: bool,
    enable_metrics: bool,
}

impl PerformanceAspectBuilder {
    pub fn new() -> Self {
        Self {
            logger: None,
            slow_threshold: Duration::from_millis(1000),
            track_memory: false,
            track_cpu: false,
            enable_metrics: true,
        }
    }

    pub fn logger(mut self, logger: Arc<dyn Logger>) -> Self {
        self.logger = Some(logger);
        self
    }

    pub fn slow_threshold(mut self, threshold: Duration) -> Self {
        self.slow_threshold = threshold;
        self
    }

    pub fn track_memory(mut self, track: bool) -> Self {
        self.track_memory = track;
        self
    }

    pub fn track_cpu(mut self, track: bool) -> Self {
        self.track_cpu = track;
        self
    }

    pub fn enable_metrics(mut self, enable: bool) -> Self {
        self.enable_metrics = enable;
        self
    }

    pub fn build(self) -> Result<PerformanceAspect> {
        let logger = self.logger.ok_or_else(|| {
            crate::errors::LoquatError::Config(crate::errors::ConfigError::MissingRequired(
                "Logger is required for performance aspect".to_string()
            ))
        })?;

        Ok(PerformanceAspect {
            logger,
            slow_threshold: self.slow_threshold,
            track_memory: self.track_memory,
            track_cpu: self.track_cpu,
            enable_metrics: self.enable_metrics,
        })
    }
}

impl std::fmt::Debug for PerformanceAspect {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PerformanceAspect")
            .field("slow_threshold", &self.slow_threshold)
            .field("track_memory", &self.track_memory)
            .field("track_cpu", &self.track_cpu)
            .field("enable_metrics", &self.enable_metrics)
            .finish()
    }
}

impl Default for PerformanceAspectBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::logging::writers::ConsoleWriter;
    use crate::logging::formatters::TextFormatter;

    #[tokio::test]
    async fn test_performance_aspect_creation() {
        let writer = Arc::new(ConsoleWriter::new());
        let formatter = Arc::new(TextFormatter::detailed());
        let logger: Arc<dyn Logger> = Arc::new(crate::logging::logger::StructuredLogger::new(formatter, writer));

        let aspect = PerformanceAspect::new(Arc::clone(&logger));
        assert_eq!(aspect.slow_threshold, Duration::from_millis(1000));
        assert!(!aspect.track_memory);
        assert!(!aspect.track_cpu);
        assert!(aspect.enable_metrics);
    }

    #[tokio::test]
    async fn test_performance_aspect_builder() {
        let writer = Arc::new(ConsoleWriter::new());
        let formatter = Arc::new(TextFormatter::detailed());
        let logger: Arc<dyn Logger> = Arc::new(crate::logging::logger::StructuredLogger::new(formatter, writer));

        let aspect = PerformanceAspectBuilder::new()
            .logger(logger)
            .slow_threshold(Duration::from_millis(500))
            .track_memory(true)
            .track_cpu(true)
            .enable_metrics(false)
            .build()
            .unwrap();

        assert_eq!(aspect.slow_threshold, Duration::from_millis(500));
        assert!(aspect.track_memory);
        assert!(aspect.track_cpu);
        assert!(!aspect.enable_metrics);
    }

    #[tokio::test]
    async fn test_performance_aspect_builder_no_logger() {
        let result = PerformanceAspectBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn test_performance_metrics() {
        let mut metrics = PerformanceMetrics::new("test_operation");

        assert!(metrics.duration().is_none());
        assert!(metrics.success.is_none());

        metrics.complete(true);

        assert!(metrics.duration().is_some());
        assert_eq!(metrics.success, Some(true));
    }
}
