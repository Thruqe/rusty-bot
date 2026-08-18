CREATE TABLE IF NOT EXISTS bot_messages (
    id VARCHAR(128) PRIMARY KEY,
    chat VARCHAR(128) NOT NULL,
    sender VARCHAR(128) NOT NULL,
    raw_message BYTEA NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_bot_messages_chat ON bot_messages(chat);
