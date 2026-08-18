CREATE TABLE IF NOT EXISTS bot_messages (
    id TEXT PRIMARY KEY,
    chat TEXT NOT NULL,
    sender TEXT NOT NULL,
    raw_message BLOB NOT NULL,
    created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS idx_bot_messages_chat ON bot_messages(chat);
