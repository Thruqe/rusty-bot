CREATE TABLE IF NOT EXISTS bot_sessions (
    id VARCHAR(64) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    category VARCHAR(50) NOT NULL DEFAULT 'default',
    phone_number VARCHAR(32),
    prefix VARCHAR(10),
    bot_name VARCHAR(100),
    auto_start BOOLEAN NOT NULL DEFAULT TRUE,
    is_active BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Seed default session if not present
INSERT INTO bot_sessions (id, name, category, auto_start, is_active)
VALUES ('default', 'Default Session', 'default', TRUE, FALSE)
ON CONFLICT (id) DO NOTHING;
