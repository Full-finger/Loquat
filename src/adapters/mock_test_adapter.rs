//! Mock Test Adapter - 定时生成测试事件的 Adapter

use crate::adapters::{
    Adapter, AdapterConfig, AdapterStatus,
    types::AdapterStatistics,
};
use crate::events::EventEnum;
use crate::events::message::MessageEvent;
use crate::events::notice::NoticeEvent;
use crate::events::traits::EventMetadata;
use crate::errors::{AdapterError, LoquatError, Result};
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Mock Test Adapter - 用于测试框架完整功能
#[derive(Debug)]
pub struct MockTestAdapter {
    config: AdapterConfig,
    status: Arc<RwLock<AdapterStatus>>,
    statistics: Arc<RwLock<AdapterStatistics>>,
    running: Arc<RwLock<bool>>,
    event_sender: Option<mpsc::UnboundedSender<EventEnum>>,
    event_counter: Arc<AtomicU64>,
    event_interval: u64,
}

impl MockTestAdapter {
    /// Create a new mock test adapter
    pub fn new(config: AdapterConfig) -> Self {
        // 从 platform 配置中读取事件间隔，默认5秒
        let event_interval = config.platform
            .get("event_interval_seconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(5);

        Self {
            config,
            status: Arc::new(RwLock::new(AdapterStatus::Ready)),
            statistics: Arc::new(RwLock::new(AdapterStatistics::default())),
            running: Arc::new(RwLock::new(false)),
            event_sender: None,
            event_counter: Arc::new(AtomicU64::new(0)),
            event_interval,
        }
    }

    /// Set the event sender channel
    pub fn set_event_sender(&mut self, sender: mpsc::UnboundedSender<EventEnum>) {
        self.event_sender = Some(sender);
    }

    /// Start the mock test adapter
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(LoquatError::Adapter(AdapterError::LoadFailed(
                "Adapter is already running".to_string()
            )));
        }

        *running = true;
        *self.status.write().await = AdapterStatus::Running;
        drop(running);

        // Spawn event generation task
        let running_clone = Arc::clone(&self.running);
        let status_clone = Arc::clone(&self.status);
        let stats_clone = Arc::clone(&self.statistics);
        let sender_clone = self.event_sender.clone();
        let counter_clone = Arc::clone(&self.event_counter);
        let adapter_id = self.config.adapter_id.clone();
        let interval_seconds = self.event_interval;

        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(interval_seconds));
            
            println!("[{}] Mock Test Adapter started", adapter_id);
            println!("[{}] Event interval: {} seconds", adapter_id, interval_seconds);
            println!("[{}] Generating events every {} seconds...", adapter_id, interval_seconds);

            while *running_clone.read().await {
                interval.tick().await;
                
                // 递增计数器
                let event_num = counter_clone.fetch_add(1, Ordering::SeqCst) + 1;
                
                // 轮换生成不同类型的事件
                let event_type = event_num % 5;
                let event = match event_type {
                    0 => Self::generate_text_event(&adapter_id, event_num),
                    1 => Self::generate_image_event(&adapter_id, event_num),
                    2 => Self::generate_notice_event(&adapter_id, event_num),
                    3 => Self::generate_voice_event(&adapter_id, event_num),
                    _ => Self::generate_group_event(&adapter_id, event_num),
                };

                println!("[{}] Generated event #{}: {:?}", adapter_id, event_num, event_type);

                // 发送事件到引擎
                if let Some(ref sender) = sender_clone {
                    if let Err(e) = sender.send(event) {
                        println!("[{}] Failed to send event to engine: {}", adapter_id, e);
                        let mut stats = stats_clone.write().await;
                        stats.errors += 1;
                    } else {
                        // 更新统计信息
                        let mut stats = stats_clone.write().await;
                        stats.events_sent += 1;
                        stats.last_activity = Some(chrono::Utc::now().timestamp());
                        drop(stats);
                    }
                } else {
                    println!("[{}] Event sender not configured, event not sent", adapter_id);
                    let mut stats = stats_clone.write().await;
                    stats.events_sent += 1;
                    stats.last_activity = Some(chrono::Utc::now().timestamp());
                    drop(stats);
                }
            }

            *status_clone.write().await = AdapterStatus::Stopped;
            println!("[{}] Mock Test Adapter stopped", adapter_id);
            println!("[{}] Total events generated: {}", adapter_id, counter_clone.load(Ordering::SeqCst));
        });

        Ok(())
    }

    /// Stop the mock test adapter
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        *self.status.write().await = AdapterStatus::Stopped;
        drop(running);

        Ok(())
    }

    /// Generate a text message event
    fn generate_text_event(adapter_id: &str, event_num: u64) -> EventEnum {
        let metadata = EventMetadata::new("message.text")
            .with_source(crate::events::EventSource::User)
            .with_user_id(&*format!("user_{}", event_num % 10));
        
        EventEnum::Message(MessageEvent::Text {
            text: format!("Test message #{} from MockTestAdapter", event_num),
            metadata,
        })
    }

    /// Generate an image message event
    fn generate_image_event(adapter_id: &str, event_num: u64) -> EventEnum {
        let metadata = EventMetadata::new("message.image")
            .with_source(crate::events::EventSource::User)
            .with_user_id(&*format!("user_{}", event_num % 10));
        
        EventEnum::Message(MessageEvent::Image {
            url: format!("http://example.com/image_{}.jpg", event_num),
            caption: Some(format!("[Image #{}] Test image from MockTestAdapter", event_num)),
            metadata,
        })
    }

    /// Generate a notice event
    fn generate_notice_event(adapter_id: &str, event_num: u64) -> EventEnum {
        let metadata = EventMetadata::new("notice.system")
            .with_source(crate::events::EventSource::System);
        
        EventEnum::Notice(NoticeEvent::SystemNotice {
            notice_type: format!("notification_{}", event_num % 3),
            content: format!("System notification #{} from MockTestAdapter", event_num),
            metadata,
        })
    }

    /// Generate a voice message event
    fn generate_voice_event(adapter_id: &str, event_num: u64) -> EventEnum {
        let metadata = EventMetadata::new("message.voice")
            .with_source(crate::events::EventSource::User)
            .with_user_id(&*format!("user_{}", event_num % 10));
        
        EventEnum::Message(MessageEvent::Voice {
            url: format!("http://example.com/voice_{}.mp3", event_num),
            duration: 10 + (event_num % 30) as u32,
            metadata,
        })
    }

    /// Generate a group message event
    fn generate_group_event(adapter_id: &str, event_num: u64) -> EventEnum {
        let metadata = EventMetadata::new("message.text")
            .with_source(crate::events::EventSource::User)
            .with_user_id(&*format!("user_{}", event_num % 10))
            .with_group_id(&*format!("group_{}", event_num % 5));
        
        EventEnum::Message(MessageEvent::Text {
            text: format!("Group message #{} from MockTestAdapter in group {}", 
                event_num, event_num % 5),
            metadata,
        })
    }
}

impl Adapter for MockTestAdapter {
    fn name(&self) -> &str {
        "MockTestAdapter"
    }

    fn version(&self) -> &str {
        "1.0.0"
    }

    fn adapter_id(&self) -> &str {
        &self.config.adapter_id
    }

    fn config(&self) -> AdapterConfig {
        self.config.clone()
    }

    fn status(&self) -> AdapterStatus {
        tokio::task::block_in_place(|| {
            let guard = tokio::runtime::Handle::current()
                .block_on(self.status.read());
            guard.clone()
        })
    }

    fn is_running(&self) -> bool {
        self.status() == AdapterStatus::Running
    }

    fn is_connected(&self) -> bool {
        self.status().is_active()
    }

    fn statistics(&self) -> AdapterStatistics {
        tokio::task::block_in_place(|| {
            let guard = tokio::runtime::Handle::current()
                .block_on(self.statistics.read());
            let mut stats = guard.clone();
            // 添加事件计数器到统计信息
            stats.events_sent = self.event_counter.load(Ordering::SeqCst);
            stats
        })
    }

    fn set_event_sender(&self, sender: Option<mpsc::UnboundedSender<EventEnum>>) {
        // This is a bit awkward because we need interior mutability
        // For now, we'll just note this limitation
        println!("set_event_sender called but not implemented due to interior mutability requirements");
    }

    fn send_event(&self, event: EventEnum) -> Result<()> {
        if let Some(ref sender) = self.event_sender {
            sender.send(event).map_err(|e| {
                LoquatError::Adapter(AdapterError::LoadFailed(format!(
                    "Failed to send event: {}", e
                )))
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_mock_test_adapter_creation() {
        let config = AdapterConfig::new("mock_test", "mock-test-001", "mock://test");
        let adapter = MockTestAdapter::new(config);

        assert_eq!(adapter.name(), "MockTestAdapter");
        assert_eq!(adapter.version(), "1.0.0");
        assert_eq!(adapter.adapter_id(), "mock-test-001");
        assert_eq!(adapter.status(), AdapterStatus::Ready);
        assert!(!adapter.is_running());
        assert_eq!(adapter.event_interval, 5);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_mock_test_adapter_custom_interval() {
        let config = AdapterConfig::new("mock_test", "mock-test-002", "mock://test")
            .with_platform_config("event_interval_seconds", 10).unwrap();
        let adapter = MockTestAdapter::new(config);

        assert_eq!(adapter.event_interval, 10);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_mock_test_adapter_statistics() {
        let config = AdapterConfig::new("mock_test", "mock-test-003", "mock://test");
        let adapter = MockTestAdapter::new(config);

        let stats = adapter.statistics();
        assert_eq!(stats.events_sent, 0); // 事件计数器初始为0
        assert_eq!(stats.events_received, 0);
        assert_eq!(stats.messages_sent, 0);
        assert_eq!(stats.errors, 0);
    }

    #[test]
    fn test_generate_text_event() {
        let event = MockTestAdapter::generate_text_event("test-adapter", 1);
        if let EventEnum::Message(msg) = event {
            if let MessageEvent::Text { text, .. } = msg {
                assert!(text.contains("Test message #1"));
            } else {
                panic!("Expected Text message event");
            }
        } else {
            panic!("Expected Message event");
        }
    }

    #[test]
    fn test_generate_image_event() {
        let event = MockTestAdapter::generate_image_event("test-adapter", 2);
        if let EventEnum::Message(msg) = event {
            if let MessageEvent::Image { caption, .. } = msg {
                if let Some(c) = caption {
                    assert!(c.contains("[Image #2]"));
                } else {
                    panic!("Expected caption in Image event");
                }
            } else {
                panic!("Expected Image message event");
            }
        } else {
            panic!("Expected Message event");
        }
    }

    #[test]
    fn test_generate_notice_event() {
        let event = MockTestAdapter::generate_notice_event("test-adapter", 3);
        if let EventEnum::Notice(notice) = event {
            if let NoticeEvent::SystemNotice { content, .. } = notice {
                assert!(content.contains("System notification #3"));
            } else {
                panic!("Expected SystemNotice event");
            }
        } else {
            panic!("Expected Notice event");
        }
    }
}
