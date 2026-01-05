# Event System Improvement

## Overview

The Loquat event system has been significantly improved with a new unified architecture that separates events into two categories:

1. **Simple Events**: Events without complex content (e.g., heartbeat, lifecycle, system notifications)
2. **Group Events**: Events with payload content (e.g., messages, notices, requests)

This new design reduces the need to work with specific event types and provides a more unified approach to event handling.

## Key Improvements

### 1. Reduced Code Duplication
- Eliminated repetitive match patterns across multiple event types
- Unified accessors for common event properties (event_id, timestamp, source, etc.)
- No more type conversions (EventSource stays as enum, not String)

### 2. Better Abstraction
- Most code can now work with the generic `UnifiedEvent` type
- Specific event types (MessageEvent, NoticeEvent, etc.) are only needed for event creation
- Cleaner separation between event definition and event handling

### 3. Improved Extensibility
- Adding new event subtypes only requires updating the corresponding Content enum
- No need to modify the main Event structure
- Easier to maintain and extend

### 4. Unified Interface
- Both simple and group events share the same metadata structure
- Consistent access patterns across all event types
- Better serialization support with tagged enums

## New Structure

### UnifiedEvent Enum

```rust
pub enum UnifiedEvent {
    Simple(SimpleEvent),
    Group(GroupEvent),
}

pub enum GroupEvent {
    Message(MessagePayload),
    Notice(NoticePayload),
    Request(RequestPayload),
}
```

### Simple Event

Used for events that don't need complex content:

```rust
pub struct SimpleEvent {
    pub event_type: String,
    pub metadata: EventMetadata,
}
```

### Group Event Payloads

Each group event type has its own payload structure:

```rust
pub struct MessagePayload {
    pub subtype: MessageSubtype,
    pub content: MessageContent,
    pub metadata: EventMetadata,
}

pub struct NoticePayload {
    pub subtype: NoticeSubtype,
    pub content: NoticeContent,
    pub metadata: EventMetadata,
}

pub struct RequestPayload {
    pub subtype: RequestSubtype,
    pub content: RequestContent,
    pub metadata: EventMetadata,
}
```

## Usage Examples

### Creating Events

#### Simple Events

```rust
use loquat::events::{UnifiedEvent, EventMetadata, EventSource};

// Heartbeat event
let heartbeat = UnifiedEvent::heartbeat(
    5000,
    EventMetadata::new("heartbeat")
);

// Lifecycle event
let lifecycle = UnifiedEvent::lifecycle(
    "started",
    EventMetadata::new("lifecycle")
);

// Custom simple event
let custom_event = UnifiedEvent::simple(
    "custom.event.type",
    EventMetadata::new("custom")
        .with_source(EventSource::System)
);
```

#### Message Events

```rust
use loquat::events::{UnifiedEvent, EventMetadata};

// Text message
let text_message = UnifiedEvent::message_text(
    "Hello, world!",
    EventMetadata::new("message.text")
        .with_user_id("user123")
        .with_group_id("group456")
);

// Image message
let image_message = UnifiedEvent::message_image(
    "https://example.com/image.jpg",
    Some("A beautiful image".to_string()),
    EventMetadata::new("message.image")
);
```

#### Notice Events

```rust
use loquat::events::{UnifiedEvent, EventMetadata, UserInfo};

// Group member join
let member_join = UnifiedEvent::notice_member_join(
    "user123".to_string(),
    "group456".to_string(),
    Some(UserInfo {
        nickname: Some("Alice".to_string()),
        avatar: None,
        card: None,
        sex: None,
        age: None,
    }),
    EventMetadata::new("notice.group.member.join")
);
```

#### Request Events

```rust
use loquat::events::{UnifiedEvent, EventMetadata};

// Friend request
let friend_request = UnifiedEvent::request_friend(
    "user123".to_string(),
    Some("Let's be friends".to_string()),
    EventMetadata::new("request.friend")
        .with_user_id("user123")
);
```

### Processing Events

#### Generic Event Handler

```rust
use loquat::events::{UnifiedEvent, MessageContent};

fn handle_event(event: &UnifiedEvent) {
    // Get basic information
    println!("Event ID: {}", event.event_id());
    println!("Event Type: {}", event.event_type());
    println!("Timestamp: {}", event.timestamp());
    
    // Check event kind
    if event.is_simple() {
        println!("This is a simple event");
    } else if event.is_group() {
        println!("This is a group event");
        
        // Check specific group event type
        if event.is_message() {
            if let Some(payload) = event.as_message() {
                if let MessageContent::Text { text } = &payload.content {
                    println!("Message: {}", text);
                }
            }
        } else if event.is_notice() {
            println!("This is a notice event");
        } else if event.is_request() {
            println!("This is a request event");
        }
    }
}
```

#### Specialized Handlers

```rust
// Message handler
fn handle_message(event: &UnifiedEvent) {
    if let Some(payload) = event.as_message() {
        match &payload.content {
            MessageContent::Text { text } => {
                println!("Text message: {}", text);
            }
            MessageContent::Image { url, caption } => {
                println!("Image: {}, caption: {:?}", url, caption);
            }
            MessageContent::At { text, at_list } => {
                println!("@ message: {}, mentions: {:?}", text, at_list);
            }
            _ => {
                println!("Other message type");
            }
        }
    }
}

// Notice handler
fn handle_notice(event: &UnifiedEvent) {
    if let Some(payload) = event.as_notice() {
        println!("Notice event: {}", payload.event_type());
        match &payload.content {
            NoticeContent::GroupMemberJoin { user_id, group_id, .. } => {
                println!("User {} joined group {}", user_id, group_id);
            }
            NoticeContent::FriendAdd { user_id, .. } => {
                println!("New friend: {}", user_id);
            }
            _ => {}
        }
    }
}
```

### Accessing Event Properties

```rust
fn process_event(event: &UnifiedEvent) {
    // Common properties available for all event types
    let event_id = event.event_id();
    let event_type = event.event_type();
    let timestamp = event.timestamp();
    let source = event.source();
    let user_id = event.user_id();
    let group_id = event.group_id();
    let self_id = event.self_id();
    let correlation_id = event.correlation_id();
    
    println!("Processing event: {}", event_type);
    println!("From user: {:?}", user_id);
    println!("In group: {:?}", group_id);
}
```

### Serialization

The new event system supports full serialization/deserialization:

```rust
use loquat::events::{UnifiedEvent, EventMetadata};

// Create event
let event = UnifiedEvent::message_text(
    "Hello",
    EventMetadata::new("message.text")
);

// Serialize to JSON
let json = serde_json::to_string(&event).unwrap();
println!("JSON: {}", json);

// Deserialize from JSON
let deserialized: UnifiedEvent = serde_json::from_str(&json).unwrap();
assert_eq!(event, deserialized);
```

## Migration Guide

### Old Code

```rust
use loquat::events::MessageEvent;
use loquat::events::traits::EventMetadata;

let message = MessageEvent::Text {
    text: "Hello".to_string(),
    metadata: EventMetadata::new("message.text"),
};

// Match on event type
match message {
    MessageEvent::Text { text, .. } => {
        println!("Text: {}", text);
    }
    MessageEvent::Image { url, .. } => {
        println!("Image: {}", url);
    }
    // ... many more variants
}
```

### New Code

```rust
use loquat::events::{UnifiedEvent, EventMetadata};

let message = UnifiedEvent::message_text(
    "Hello",
    EventMetadata::new("message.text")
);

// Access content directly
if let Some(payload) = message.as_message() {
    if let MessageContent::Text { text } = &payload.content {
        println!("Text: {}", text);
    }
}
```

## Benefits Summary

1. **Less Boilerplate**: No need to match on every event variant to get basic properties
2. **Type Safety**: EventSource and other enums maintain their types
3. **Better Organization**: Clear separation between simple and group events
4. **Easier Testing**: Simpler event creation and verification
5. **Future-Proof**: Easy to add new event types without breaking existing code

## Backward Compatibility

The old event types (MessageEvent, NoticeEvent, RequestEvent, MetaEvent) are still available for backward compatibility. However, new code should prefer using UnifiedEvent for most scenarios.

You can convert between old and new formats as needed:

```rust
// Old to new (manual conversion)
let old_message = MessageEvent::Text { /* ... */ };
// Convert to UnifiedEvent::Group(GroupEvent::Message(...))

// New to old (if needed)
let unified = UnifiedEvent::message_text("Hello", metadata);
if let Some(payload) = unified.as_message() {
    // Access payload data and create old-style event if needed
}
```

## Testing

The new event system includes comprehensive tests:

```bash
# Run event tests
cargo test --lib events

# Run specific test
cargo test test_simple_event_creation
cargo test test_message_text_event
cargo test test_serialization
```

## Future Enhancements

Potential future improvements to the event system:

1. **Event Filtering**: Built-in event filtering and routing capabilities
2. **Event Transformation**: Support for event transformation pipelines
3. **Event Validation**: Schema validation for event payloads
4. **Event Aggregation**: Support for aggregating multiple events
5. **Event Replay**: Event replay and auditing capabilities
