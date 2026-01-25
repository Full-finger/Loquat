-- Loquat Framework Database Schema
-- SQLite schema version 1.0.0

-- Events table
CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    package_id TEXT NOT NULL UNIQUE,
    event_type TEXT NOT NULL,
    source TEXT NOT NULL,
    target_site TEXT NOT NULL,
    timestamp TEXT NOT NULL,
    raw_data TEXT NOT NULL,
    status INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for events table
CREATE INDEX IF NOT EXISTS idx_events_package_id ON events(package_id);
CREATE INDEX IF NOT EXISTS idx_events_event_type ON events(event_type);
CREATE INDEX IF NOT EXISTS idx_events_source ON events(source);
CREATE INDEX IF NOT EXISTS idx_events_timestamp ON events(timestamp);
CREATE INDEX IF NOT EXISTS idx_events_status ON events(status);
CREATE INDEX IF NOT EXISTS idx_events_created_at ON events(created_at);

-- Plugins table
CREATE TABLE IF NOT EXISTS plugins (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL UNIQUE,
    version TEXT NOT NULL,
    plugin_type TEXT NOT NULL,
    file_path TEXT NOT NULL,
    status INTEGER NOT NULL DEFAULT 0,
    last_loaded_at TEXT,
    load_count INTEGER NOT NULL DEFAULT 0,
    error_message TEXT,
    metadata TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for plugins table
CREATE INDEX IF NOT EXISTS idx_plugins_name ON plugins(name);
CREATE INDEX IF NOT EXISTS idx_plugins_status ON plugins(status);
CREATE INDEX IF NOT EXISTS idx_plugins_plugin_type ON plugins(plugin_type);
CREATE INDEX IF NOT EXISTS idx_plugins_updated_at ON plugins(updated_at);

-- Adapters table
CREATE TABLE IF NOT EXISTS adapters (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    adapter_id TEXT NOT NULL UNIQUE,
    adapter_type TEXT NOT NULL,
    config TEXT NOT NULL,
    status INTEGER NOT NULL DEFAULT 0,
    connected INTEGER NOT NULL DEFAULT 0,
    last_started_at TEXT,
    last_stopped_at TEXT,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for adapters table
CREATE INDEX IF NOT EXISTS idx_adapters_adapter_id ON adapters(adapter_id);
CREATE INDEX IF NOT EXISTS idx_adapters_adapter_type ON adapters(adapter_type);
CREATE INDEX IF NOT EXISTS idx_adapters_status ON adapters(status);
CREATE INDEX IF NOT EXISTS idx_adapters_connected ON adapters(connected);
CREATE INDEX IF NOT EXISTS idx_adapters_updated_at ON adapters(updated_at);

-- Logs table
CREATE TABLE IF NOT EXISTS logs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    level TEXT NOT NULL,
    message TEXT NOT NULL,
    component TEXT,
    timestamp TEXT NOT NULL,
    context TEXT
);

-- Indexes for logs table
CREATE INDEX IF NOT EXISTS idx_logs_level ON logs(level);
CREATE INDEX IF NOT EXISTS idx_logs_component ON logs(component);
CREATE INDEX IF NOT EXISTS idx_logs_timestamp ON logs(timestamp);

-- Statistics table
CREATE TABLE IF NOT EXISTS stats (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,
    value TEXT NOT NULL,
    timestamp TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for stats table
CREATE INDEX IF NOT EXISTS idx_stats_name ON stats(name);
CREATE INDEX IF NOT EXISTS idx_stats_timestamp ON stats(timestamp);

-- Triggers for automatic timestamp updates
CREATE TRIGGER IF NOT EXISTS update_plugins_timestamp
AFTER UPDATE ON plugins
BEGIN
    UPDATE plugins SET updated_at = datetime('now') WHERE id = NEW.id;
END;

CREATE TRIGGER IF NOT EXISTS update_adapters_timestamp
AFTER UPDATE ON adapters
BEGIN
    UPDATE adapters SET updated_at = datetime('now') WHERE id = NEW.id;
END;
