CREATE TABLE IF NOT EXISTS bot_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    prefix TEXT NOT NULL DEFAULT '.',
    bot_name TEXT NOT NULL DEFAULT 'WaBot',
    work_type TEXT NOT NULL DEFAULT 'public', -- 'public' | 'private'
    auto_read INTEGER NOT NULL DEFAULT 0,
    auto_typing INTEGER NOT NULL DEFAULT 0,
    auto_recording INTEGER NOT NULL DEFAULT 0,
    reject_call INTEGER NOT NULL DEFAULT 0,
    always_online INTEGER NOT NULL DEFAULT 1,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Seed single default configuration row if empty
INSERT OR IGNORE INTO bot_settings (id, prefix, bot_name, work_type)
VALUES (1, '.', 'WaBot', 'public');