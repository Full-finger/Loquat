# Adapters Directory

This directory contains adapter configuration files for the Loquat framework.

## Overview

Adapters are the bridge between the Loquat framework and external messaging platforms (QQ, WeChat, Telegram, etc.). Each adapter has its own configuration file in JSON format.

## File Format

Each adapter configuration file is a JSON document with the following structure:

```json
{
  "adapter_type": "string",      // Required: Type of adapter
  "adapter_id": "string",        // Required: Unique identifier
  "enabled": boolean,            // Optional: Whether adapter is enabled (default: true)
  "name": "string",              // Optional: Display name
  "connection": {                 // Required: Connection settings
    "conn_type": "string",       // Connection type (ws, http, tcp, stdio, echo, mock)
    "url": "string",             // Connection URL or address
    "timeout": number,           // Timeout in seconds (default: 30)
    "use_tls": boolean,          // Whether to use TLS (default: false)
    "keep_alive": number|null,   // Keep-alive interval in seconds
    "max_reconnect": number,     // Maximum reconnection attempts (default: 5)
    "params": {}                 // Additional connection parameters
  },
  "heartbeat": {                 // Optional: Heartbeat configuration
    "interval": number,          // Heartbeat interval in seconds
    "timeout": number|null,      // Heartbeat timeout in seconds
    "enabled": boolean           // Whether heartbeat is enabled (default: true)
  },
  "retry": {                     // Optional: Retry configuration
    "max_attempts": number,      // Maximum retry attempts (default: 3)
    "initial_delay": number,     // Initial delay in ms (default: 1000)
    "max_delay": number,         // Maximum delay in ms (default: 30000)
    "backoff_multiplier": number // Backoff multiplier (default: 2.0)
  },
  "platform": {},                // Optional: Platform-specific settings
  "extra": {}                   // Optional: Additional metadata
}
```

## Built-in Adapters

### Console Adapter (`console.json`)
Reads input from stdin and outputs to stdout. Useful for testing and development.

Configuration example:
```json
{
  "adapter_type": "console",
  "adapter_id": "console-001",
  "connection": {
    "conn_type": "stdio",
    "url": "stdio://"
  }
}
```

### Echo Adapter (`echo.json`)
Echoes back received messages. Simple adapter for testing basic functionality.

Configuration example:
```json
{
  "adapter_type": "echo",
  "adapter_id": "echo-001",
  "connection": {
    "conn_type": "echo",
    "url": "echo://"
  }
}
```

### Mock Test Adapter (`mock_test.json`)
Generates test events at regular intervals. Useful for testing event processing and routing.

Configuration example:
```json
{
  "adapter_type": "mock_test",
  "adapter_id": "mock-test-001",
  "connection": {
    "conn_type": "mock",
    "url": "mock://test"
  },
  "platform": {
    "event_interval_seconds": 5
  }
}
```

Platform-specific settings:
- `event_interval_seconds`: Interval between generated events (default: 5)

## Adding External Adapters

To add a new external adapter:

1. **Create a configuration file** in this directory with a descriptive name (e.g., `qq-bot.json`)

2. **Configure the adapter** with appropriate settings:
   ```json
   {
     "adapter_type": "qq",
     "adapter_id": "qq-bot-001",
     "connection": {
       "conn_type": "ws",
       "url": "ws://localhost:8080"
     },
     "platform": {
       "app_id": "your_app_id",
       "app_secret": "your_app_secret"
     }
   }
   ```

3. **Implement the adapter factory** in `src/adapters/`:
   - Create a file for your adapter implementation
   - Implement the `Adapter` trait
   - Implement the `AdapterFactory` trait
   - Register the factory in `main.rs`

4. **Restart the framework** to load the new adapter

## Configuration Management

### White/Black List

You can control which adapters are loaded using the whitelist and blacklist in the main configuration:

```toml
[adapters]
whitelist = ["console", "qq"]  # Only load these adapters
blacklist = ["echo"]            # Don't load these adapters
```

**Priority**: Blacklist takes precedence over whitelist.

### Auto-Loading

Adapters are automatically loaded on startup if `auto_load` is enabled in the configuration:

```toml
[adapters]
enabled = true
auto_load = true
adapter_dir = "./adapters"
```

### Hot Reload

Hot reload allows adapters to be reloaded automatically when their configuration files change:

```toml
[adapters]
enable_hot_reload = true
hot_reload_interval = 10  # Check every 10 seconds
```

## Supported File Formats

- JSON files (`.json`) - Recommended, fully supported
- YAML files (`.yaml`, `.yml`) - Support planned

## Adapter States

Adapters can be in one of the following states:
- **Uninitialized**: Not yet initialized
- **Initializing**: Being initialized
- **Ready**: Ready to start
- **Running**: Active and processing events
- **Paused**: Temporarily paused
- **Stopped**: Stopped gracefully
- **Error**: An error occurred

## Troubleshooting

### Adapter Not Loading

1. Check if the configuration file is valid JSON
2. Verify the `adapter_type` matches a registered factory
3. Ensure the adapter is not in the blacklist
4. Check the framework logs for error messages

### Adapter Not Starting

1. Verify the adapter is enabled (`"enabled": true`)
2. Check connection settings (URL, timeout, etc.)
3. Review logs for connection errors
4. Ensure required dependencies are available

### Hot Reload Not Working

1. Verify `enable_hot_reload` is set to `true`
2. Check the `hot_reload_interval` is reasonable (minimum 1 second)
3. Ensure the framework has write access to the configuration files
4. Check if file modifications are being detected (file system dependent)

## Best Practices

1. **Use descriptive adapter IDs**: e.g., `qq-bot-production`, `wechat-test`
2. **Keep configuration files organized**: Group related adapters
3. **Document custom platform settings**: Add comments in your adapter implementation
4. **Use environment-specific configurations**: Separate dev/test/prod configs
5. **Monitor adapter health**: Use the health check mechanisms provided

## Security Considerations

1. **Never commit secrets**: Use environment variables or secret management
2. **Validate external input**: Sanitize all incoming messages
3. **Use TLS for production**: Enable `use_tls` for secure connections
4. **Limit retry attempts**: Prevent infinite retry loops
5. **Monitor for anomalies**: Set up alerts for unusual adapter behavior

## Support

For more information about:
- Adapter implementation: See `src/adapters/` directory
- Configuration options: See `config/` directory
- Framework documentation: See project README
- Issues and bugs: Report to project issue tracker
