CREATE TABLE IF NOT EXISTS bot_sessions (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    category TEXT NOT NULL DEFAULT 'default',
    phone_number TEXT,
    prefix TEXT,
    bot_name TEXT,
    auto_start INTEGER NOT NULL DEFAULT 1,
    is_active INTEGER NOT NULL DEFAULT 0,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Seed default session if not present
INSERT OR IGNORE INTO bot_sessions (id, name, category, auto_start, is_active)
VALUES ('default', 'Default Session', 'default', 1, 0);
