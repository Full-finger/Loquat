//! NapCat Adapter - communicates with NapCat QQ bot framework via OneBot protocol
//!
//! This adapter implements WebSocket communication with NapCat, which is based on
//! the OneBot standard. It receives events from NapCat and converts them to Loquat
//! events, and can send messages back to NapCat.

use crate::adapters::core::{
    Adapter, AdapterConfig, AdapterStatus,
    types::AdapterStatistics,
};
use crate::events::{EventEnum, EventMetadata, EventSource};
use crate::events::message::MessageEvent;
use crate::errors::{AdapterError, LoquatError, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tokio_tungstenite::{
    connect_async_with_config,
    tungstenite::protocol::Message as WsMessage,
};
use futures_util::{StreamExt, SinkExt};

/// NapCat OneBot event types
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "post_type")]
#[serde(rename_all = "snake_case")]
enum OneBotEvent {
    #[serde(rename = "message")]
    Message(MessageEventData),
    #[serde(rename = "notice")]
    Notice(NoticeEventData),
    #[serde(rename = "request")]
    Request(RequestEventData),
    #[serde(rename = "meta_event")]
    MetaEvent(MetaEventData),
}

/// Message event data from OneBot
#[derive(Debug, Deserialize, Serialize)]
struct MessageEventData {
    #[serde(rename = "time")]
    timestamp: i64,
    #[serde(rename = "self_id")]
    self_id: String,
    #[serde(rename = "post_type")]
    post_type: String,
    #[serde(rename = "message_type")]
    message_type: String,
    #[serde(rename = "sub_type")]
    sub_type: Option<String>,
    #[serde(rename = "message_id")]
    message_id: String,
    #[serde(rename = "user_id")]
    user_id: i64,
    #[serde(rename = "group_id")]
    group_id: Option<i64>,
    #[serde(rename = "sender")]
    sender: SenderData,
    #[serde(rename = "raw_message")]
    raw_message: String,
    #[serde(rename = "message")]
    message: Vec<MessageSegment>,
    #[serde(rename = "font")]
    font: Option<i32>,
    #[serde(rename = "reply")]
    reply: Option<ReplyData>,
}

/// Notice event data from OneBot
#[derive(Debug, Deserialize, Serialize)]
struct NoticeEventData {
    #[serde(rename = "time")]
    timestamp: i64,
    #[serde(rename = "self_id")]
    self_id: String,
    #[serde(rename = "post_type")]
    post_type: String,
    #[serde(rename = "notice_type")]
    notice_type: String,
    #[serde(rename = "sub_type")]
    sub_type: Option<String>,
    #[serde(rename = "group_id")]
    group_id: Option<i64>,
    #[serde(rename = "user_id")]
    user_id: Option<i64>,
    #[serde(rename = "operator_id")]
    operator_id: Option<i64>,
    #[serde(rename = "card")]
    card: Option<String>,
    #[serde(rename = "sex")]
    sex: Option<String>,
    #[serde(rename = "age")]
    age: Option<i32>,
    #[serde(rename = "nickname")]
    nickname: Option<String>,
}

/// Request event data from OneBot
#[derive(Debug, Deserialize, Serialize)]
struct RequestEventData {
    #[serde(rename = "time")]
    timestamp: i64,
    #[serde(rename = "self_id")]
    self_id: String,
    #[serde(rename = "post_type")]
    post_type: String,
    #[serde(rename = "request_type")]
    request_type: String,
    #[serde(rename = "sub_type")]
    sub_type: Option<String>,
    #[serde(rename = "group_id")]
    group_id: Option<i64>,
    #[serde(rename = "user_id")]
    user_id: Option<i64>,
    #[serde(rename = "comment")]
    comment: Option<String>,
    #[serde(rename = "flag")]
    flag: Option<String>,
}

/// Meta event data from OneBot
#[derive(Debug, Deserialize, Serialize)]
struct MetaEventData {
    #[serde(rename = "time")]
    timestamp: i64,
    #[serde(rename = "self_id")]
    self_id: String,
    #[serde(rename = "post_type")]
    post_type: String,
    #[serde(rename = "meta_event_type")]
    meta_event_type: String,
    #[serde(rename = "status")]
    status: StatusData,
}

/// Sender data
#[derive(Debug, Deserialize, Serialize)]
struct SenderData {
    #[serde(rename = "user_id")]
    user_id: i64,
    #[serde(rename = "nickname")]
    nickname: Option<String>,
    #[serde(rename = "card")]
    card: Option<String>,
    #[serde(rename = "sex")]
    sex: Option<String>,
    #[serde(rename = "age")]
    age: Option<i32>,
    #[serde(rename = "area")]
    area: Option<String>,
    #[serde(rename = "level")]
    level: Option<String>,
    #[serde(rename = "role")]
    role: Option<String>,
    #[serde(rename = "title")]
    title: Option<String>,
}

/// Reply data
#[derive(Debug, Deserialize, Serialize)]
struct ReplyData {
    #[serde(rename = "message_id")]
    message_id: String,
    #[serde(rename = "user_id")]
    user_id: i64,
    #[serde(rename = "time")]
    time: i64,
    #[serde(rename = "message")]
    message: Vec<MessageSegment>,
}

/// Status data
#[derive(Debug, Deserialize, Serialize)]
struct StatusData {
    #[serde(rename = "online")]
    online: bool,
    #[serde(rename = "good")]
    good: bool,
}

/// Message segment (OneBot message format)
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
enum MessageSegment {
    #[serde(rename = "text")]
    Text { data: TextData },
    #[serde(rename = "image")]
    Image { data: ImageData },
    #[serde(rename = "at")]
    At { data: AtData },
    #[serde(rename = "face")]
    Face { data: FaceData },
    #[serde(rename = "record")]
    Record { data: RecordData },
    #[serde(rename = "video")]
    Video { data: VideoData },
    #[serde(rename = "reply")]
    Reply { data: ReplySegData },
    #[serde(rename = "location")]
    Location { data: LocationData },
    #[serde(rename = "json")]
    Json { data: Value },
    #[serde(rename = "xml")]
    Xml { data: XmlData },
}

#[derive(Debug, Deserialize, Serialize)]
struct TextData {
    text: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ImageData {
    file: Option<String>,
    url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct AtData {
    qq: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct FaceData {
    id: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct RecordData {
    file: Option<String>,
    url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct VideoData {
    file: Option<String>,
    url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct ReplySegData {
    id: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct LocationData {
    lat: String,
    lon: String,
    title: Option<String>,
    content: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct XmlData {
    data: String,
}

/// NapCat adapter implementation
#[derive(Debug)]
pub struct NapCatAdapter {
    config: AdapterConfig,
    status: Arc<RwLock<AdapterStatus>>,
    statistics: Arc<RwLock<AdapterStatistics>>,
    running: Arc<RwLock<bool>>,
    event_sender: Option<mpsc::UnboundedSender<EventEnum>>,
    message_sender: Arc<RwLock<Option<mpsc::UnboundedSender<String>>>>,
}

impl NapCatAdapter {
    /// Create a new NapCat adapter
    pub fn new(config: AdapterConfig) -> Self {
        Self {
            config,
            status: Arc::new(RwLock::new(AdapterStatus::Ready)),
            statistics: Arc::new(RwLock::new(AdapterStatistics::default())),
            running: Arc::new(RwLock::new(false)),
            event_sender: None,
            message_sender: Arc::new(RwLock::new(None)),
        }
    }

    /// Convert OneBot message to Loquat event
    fn convert_message_to_event(&self, msg_data: MessageEventData) -> Option<EventEnum> {
        let adapter_id = self.config.adapter_id.clone();
        
        // Build metadata
        let mut metadata = EventMetadata::new("message.text")
            .with_source(EventSource::Worker(adapter_id.clone()))
            .with_self_id(&msg_data.self_id)
            .with_user_id(&msg_data.user_id.to_string());
        
        if let Some(group_id) = msg_data.group_id {
            metadata = metadata.with_group_id(&group_id.to_string());
        }

        // Extract text from message segments
        let text_content = self.extract_text_from_segments(&msg_data.message);
        
        // Create message event
        let message_event = MessageEvent::Text {
            text: text_content,
            metadata,
        };

        Some(EventEnum::Message(message_event))
    }

    /// Extract plain text from message segments
    fn extract_text_from_segments(&self, segments: &[MessageSegment]) -> String {
        segments
            .iter()
            .filter_map(|seg| match seg {
                MessageSegment::Text { data } => Some(data.text.clone()),
                MessageSegment::At { data } => Some(format!("@{}", data.qq)),
                _ => None,
            })
            .collect()
    }

    /// Send message through WebSocket
    pub async fn send_message(&self, target: &str, message: &str) -> Result<()> {
        let sender_guard = self.message_sender.read().await;
        let sender = sender_guard.as_ref().ok_or_else(|| {
            LoquatError::Adapter(AdapterError::InitFailed(
                "WebSocket not connected".to_string()
            ))
        })?;

        // Build OneBot API call
        let api_call = serde_json::json!({
            "action": "send_msg",
            "params": {
                "message_type": if target.starts_with("group:") {
                    "group"
                } else {
                    "private"
                },
                if target.starts_with("group:") {
                    "group_id"
                } else {
                    "user_id"
                }: target.replace("group:", "").replace("user:", ""),
                "message": [
                    {
                        "type": "text",
                        "data": {
                            "text": message
                        }
                    }
                ]
            }
        });

        sender.send(api_call.to_string()).map_err(|e| {
            LoquatError::Adapter(AdapterError::InitFailed(
                format!("Failed to send message: {}", e)
            ))
        })?;

        // Update statistics
        let mut stats = self.statistics.write().await;
        stats.messages_sent += 1;
        stats.last_activity = Some(chrono::Utc::now().timestamp());
        drop(stats);

        Ok(())
    }

    /// Start the NapCat adapter
    pub async fn start(&self) -> Result<()> {
        let mut running = self.running.write().await;
        if *running {
            return Err(LoquatError::Adapter(AdapterError::LoadFailed(
                "Adapter is already running".to_string()
            )));
        }

        *running = true;
        *self.status.write().await = AdapterStatus::Initializing;
        drop(running);

        let url = self.config.connection.url.clone();
        let adapter_id = self.config.adapter_id.clone();
        
        // Clone for use in spawned task
        let running_clone = Arc::clone(&self.running);
        let status_clone = Arc::clone(&self.status);
        let stats_clone = Arc::clone(&self.statistics);
        let message_sender_clone = Arc::clone(&self.message_sender);
        let event_sender_clone = self.event_sender.clone();

        tokio::spawn(async move {
            let url_str = url.clone();
            let adapter_id_clone = adapter_id.clone();

            // Establish WebSocket connection
            let ws_stream = match connect_async_with_config(&url_str, None, false).await {
                Ok((stream, _)) => {
                    tracing::info!("[{}] Connected to NapCat server", adapter_id_clone);
                    *status_clone.write().await = AdapterStatus::Running;
                    stream
                }
                Err(e) => {
                    tracing::error!("[{}] Failed to connect to NapCat: {}", adapter_id_clone, e);
                    *status_clone.write().await = AdapterStatus::Error(e.to_string());
                    *running_clone.write().await = false;
                    return;
                }
            };

            let (mut write, mut read) = ws_stream.split();

            // Create a channel for sending messages
            let (msg_tx, mut msg_rx) = mpsc::unbounded_channel::<String>();
            *message_sender_clone.write().await = Some(msg_tx);

            // Spawn a task to handle outgoing messages
            let running_write = Arc::clone(&running_clone);
            tokio::spawn(async move {
                while *running_write.read().await {
                    match msg_rx.recv().await {
                        Some(msg) => {
                            if let Err(e) = write.send(WsMessage::Text(msg)).await {
                                tracing::error!("Failed to send WebSocket message: {}", e);
                                break;
                            }
                        }
                        None => {
                            tracing::info!("Message sender closed");
                            break;
                        }
                    }
                }
            });

            // Process incoming messages
            while *running_clone.read().await {
                match read.next().await {
                    Some(Ok(WsMessage::Text(text))) => {
                        // Parse OneBot event
                        match serde_json::from_str::<OneBotEvent>(&text) {
                            Ok(event) => {
                                match event {
                                    OneBotEvent::Message(msg_data) => {
                                        tracing::debug!("[{}] Received message event", adapter_id_clone);
                                        
                                        // Update statistics
                                        let mut stats = stats_clone.write().await;
                                        stats.events_received += 1;
                                        stats.last_activity = Some(chrono::Utc::now().timestamp());
                                        drop(stats);

                                        // Convert and send event
                                        if let Some(ref sender) = event_sender_clone {
                                            if let Some(loquat_event) = Self::convert_message_to_event_internal(
                                                &adapter_id_clone, 
                                                msg_data
                                            ) {
                                                let _ = sender.send(loquat_event);
                                            }
                                        }
                                    }
                                    OneBotEvent::Notice(notice_data) => {
                                        tracing::debug!("[{}] Received notice event: {}", 
                                            adapter_id_clone, notice_data.notice_type);
                                    }
                                    OneBotEvent::Request(req_data) => {
                                        tracing::debug!("[{}] Received request event: {}", 
                                            adapter_id_clone, req_data.request_type);
                                    }
                                    OneBotEvent::MetaEvent(meta_data) => {
                                        tracing::debug!("[{}] Received meta event: {}", 
                                            adapter_id_clone, meta_data.meta_event_type);
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("[{}] Failed to parse event: {}", adapter_id_clone, e);
                                let mut stats = stats_clone.write().await;
                                stats.errors += 1;
                                drop(stats);
                            }
                        }
                    }
                    Some(Ok(WsMessage::Close(_))) => {
                        tracing::info!("[{}] Connection closed by server", adapter_id_clone);
                        *running_clone.write().await = false;
                        *status_clone.write().await = AdapterStatus::Stopped;
                        break;
                    }
                    Some(Ok(_)) => {
                        // Ignore other message types
                    }
                    Some(Err(e)) => {
                        tracing::error!("[{}] WebSocket error: {}", adapter_id_clone, e);
                        let mut stats = stats_clone.write().await;
                        stats.errors += 1;
                        drop(stats);
                        *status_clone.write().await = AdapterStatus::Error(e.to_string());
                        break;
                    }
                    None => {
                        tracing::info!("[{}] Connection closed", adapter_id_clone);
                        *running_clone.write().await = false;
                        *status_clone.write().await = AdapterStatus::Stopped;
                        break;
                    }
                }
            }

            tracing::info!("[{}] NapCat adapter stopped", adapter_id_clone);
        });

        Ok(())
    }

    /// Static helper to convert message to event
    fn convert_message_to_event_internal(
        adapter_id: &str, 
        msg_data: MessageEventData
    ) -> Option<EventEnum> {
        let mut metadata = EventMetadata::new("message.text")
            .with_source(EventSource::Worker(adapter_id.to_string()))
            .with_self_id(&msg_data.self_id)
            .with_user_id(&msg_data.user_id.to_string());
        
        if let Some(group_id) = msg_data.group_id {
            metadata = metadata.with_group_id(&group_id.to_string());
        }

        // Extract text from message segments
        let text_content: String = msg_data
            .message
            .iter()
            .filter_map(|seg| match seg {
                MessageSegment::Text { data } => Some(data.text.clone()),
                MessageSegment::At { data } => Some(format!("@{}", data.qq)),
                _ => None,
            })
            .collect();
        
        let message_event = MessageEvent::Text {
            text: text_content,
            metadata,
        };

        Some(EventEnum::Message(message_event))
    }

    /// Stop the NapCat adapter
    pub async fn stop(&self) -> Result<()> {
        let mut running = self.running.write().await;
        *running = false;
        *self.status.write().await = AdapterStatus::Stopped;
        drop(running);

        Ok(())
    }
}

impl Adapter for NapCatAdapter {
    fn name(&self) -> &str {
        "NapCatAdapter"
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
            guard.clone()
        })
    }

    fn set_event_sender(&self, sender: Option<mpsc::UnboundedSender<EventEnum>>) {
        // Note: This is a simplified implementation
        // In a production implementation, you'd need to properly handle this
        let _ = sender;
    }

    fn send_event(&self, _event: EventEnum) -> Result<()> {
        // Events are sent directly from the WebSocket reader task
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(flavor = "multi_thread", worker_threads = 1)]
    async fn test_napcat_adapter_creation() {
        let config = AdapterConfig::new("napcat", "napcat-test-001", "ws://localhost:3001");
        let adapter = NapCatAdapter::new(config);

        assert_eq!(adapter.name(), "NapCatAdapter");
        assert_eq!(adapter.version(), "1.0.0");
        assert_eq!(adapter.adapter_id(), "napcat-test-001");
        assert_eq!(adapter.status(), AdapterStatus::Ready);
        assert!(!adapter.is_running());
    }

    #[test]
    fn test_extract_text_from_segments() {
        let config = AdapterConfig::new("napcat", "napcat-test-002", "ws://localhost:3001");
        let adapter = NapCatAdapter::new(config);

        let segments = vec![
            MessageSegment::Text {
                data: TextData { text: "Hello ".to_string() }
            },
            MessageSegment::At {
                data: AtData { qq: "123456".to_string() }
            },
            MessageSegment::Text {
                data: TextData { text: " world".to_string() }
            },
        ];

        let text = adapter.extract_text_from_segments(&segments);
        assert_eq!(text, "Hello @123456 world");
    }
}
