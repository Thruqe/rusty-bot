CREATE TABLE IF NOT EXISTS bot_settings (
    id SMALLINT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    prefix VARCHAR(10) NOT NULL DEFAULT '.',
    bot_name VARCHAR(100) NOT NULL DEFAULT 'WaBot',
    work_type VARCHAR(20) NOT NULL DEFAULT 'public', -- 'public' | 'private'
    auto_read BOOLEAN NOT NULL DEFAULT FALSE,
    auto_typing BOOLEAN NOT NULL DEFAULT FALSE,
    auto_recording BOOLEAN NOT NULL DEFAULT FALSE,
    reject_call BOOLEAN NOT NULL DEFAULT FALSE,
    always_online BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Seed single default configuration row if empty
INSERT INTO bot_settings (id, prefix, bot_name, work_type)
VALUES (1, '.', 'WaBot', 'public')
ON CONFLICT (id) DO NOTHING;