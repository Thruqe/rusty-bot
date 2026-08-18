pub mod messages;
pub mod sessions;
pub mod settings;

use async_trait::async_trait;
pub use messages::MessageStore;
pub use sessions::{SessionModel, SessionStore};
pub use settings::{BotSettings, SettingsStore};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::sqlite::{SqlitePool, SqlitePoolOptions};
use std::sync::Arc;

#[derive(Clone)]
pub enum Database {
    Sqlite(SqlitePool),
    Postgres(PgPool),
}

impl Database {
    pub async fn new_sqlite(database_url: &str) -> Result<Arc<Self>, sqlx::Error> {
        let url = if !database_url.starts_with("sqlite:") {
            format!("sqlite:{}?mode=rwc", database_url)
        } else {
            database_url.to_string()
        };
        let pool = SqlitePoolOptions::new()
            .max_connections(2)
            .min_connections(1)
            .connect(&url)
            .await?;
        // Constrain SQLite internal caching to keep memory footprint ultra-low
        let _ = sqlx::raw_sql("PRAGMA cache_size = -500; PRAGMA mmap_size = 0; PRAGMA synchronous = NORMAL;").execute(&pool).await;
        let db = Arc::new(Database::Sqlite(pool));
        db.init_tables().await?;
        Ok(db)
    }

    pub async fn new_postgres(database_url: &str) -> Result<Arc<Self>, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(2)
            .min_connections(1)
            .connect(database_url)
            .await?;
        let db = Arc::new(Database::Postgres(pool));
        db.init_tables().await?;
        Ok(db)
    }

    pub async fn init_tables(&self) -> Result<(), sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                let schema_settings = include_str!("../sql/sqlite/001_bot_settings.sql");
                sqlx::raw_sql(schema_settings).execute(pool).await?;

                let schema_sessions = include_str!("../sql/sqlite/002_sessions.sql");
                sqlx::raw_sql(schema_sessions).execute(pool).await?;

                let schema_messages = include_str!("../sql/sqlite/003_messages.sql");
                sqlx::raw_sql(schema_messages).execute(pool).await?;
            }
            Database::Postgres(pool) => {
                let schema_settings = include_str!("../sql/postgres/001_bot_settings.sql");
                sqlx::raw_sql(schema_settings).execute(pool).await?;

                let schema_sessions = include_str!("../sql/postgres/002_sessions.sql");
                sqlx::raw_sql(schema_sessions).execute(pool).await?;

                let schema_messages = include_str!("../sql/postgres/003_messages.sql");
                sqlx::raw_sql(schema_messages).execute(pool).await?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl SettingsStore for Database {
    async fn get_settings(&self) -> Result<BotSettings, sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                #[derive(sqlx::FromRow)]
                struct SqliteSettingsRow {
                    id: i64,
                    prefix: String,
                    bot_name: String,
                    work_type: String,
                    auto_read: i64,
                    auto_typing: i64,
                    auto_recording: i64,
                    reject_call: i64,
                    always_online: i64,
                }

                let row = sqlx::query_as::<_, SqliteSettingsRow>(
                    "SELECT id, prefix, bot_name, work_type, auto_read, auto_typing, auto_recording, reject_call, always_online FROM bot_settings WHERE id = 1",
                )
                .fetch_one(pool)
                .await?;

                Ok(BotSettings {
                    id: row.id,
                    prefix: row.prefix,
                    bot_name: row.bot_name,
                    work_type: row.work_type,
                    auto_read: row.auto_read != 0,
                    auto_typing: row.auto_typing != 0,
                    auto_recording: row.auto_recording != 0,
                    reject_call: row.reject_call != 0,
                    always_online: row.always_online != 0,
                })
            }
            Database::Postgres(pool) => {
                let row = sqlx::query_as::<_, (i16, String, String, String, bool, bool, bool, bool, bool)>(
                    "SELECT id, prefix, bot_name, work_type, auto_read, auto_typing, auto_recording, reject_call, always_online FROM bot_settings WHERE id = 1",
                )
                .fetch_one(pool)
                .await?;

                Ok(BotSettings {
                    id: row.0 as i64,
                    prefix: row.1,
                    bot_name: row.2,
                    work_type: row.3,
                    auto_read: row.4,
                    auto_typing: row.5,
                    auto_recording: row.6,
                    reject_call: row.7,
                    always_online: row.8,
                })
            }
        }
    }

    async fn set_prefix(&self, prefix: &str) -> Result<(), sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                sqlx::query("UPDATE bot_settings SET prefix = ?, updated_at = CURRENT_TIMESTAMP WHERE id = 1")
                    .bind(prefix)
                    .execute(pool)
                    .await?;
            }
            Database::Postgres(pool) => {
                sqlx::query("UPDATE bot_settings SET prefix = $1, updated_at = CURRENT_TIMESTAMP WHERE id = 1")
                    .bind(prefix)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    async fn set_bot_name(&self, name: &str) -> Result<(), sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                sqlx::query("UPDATE bot_settings SET bot_name = ?, updated_at = CURRENT_TIMESTAMP WHERE id = 1")
                    .bind(name)
                    .execute(pool)
                    .await?;
            }
            Database::Postgres(pool) => {
                sqlx::query("UPDATE bot_settings SET bot_name = $1, updated_at = CURRENT_TIMESTAMP WHERE id = 1")
                    .bind(name)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    async fn set_work_type(&self, work_type: &str) -> Result<(), sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                sqlx::query("UPDATE bot_settings SET work_type = ?, updated_at = CURRENT_TIMESTAMP WHERE id = 1")
                    .bind(work_type)
                    .execute(pool)
                    .await?;
            }
            Database::Postgres(pool) => {
                sqlx::query("UPDATE bot_settings SET work_type = $1, updated_at = CURRENT_TIMESTAMP WHERE id = 1")
                    .bind(work_type)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    async fn set_auto_read(&self, enabled: bool) -> Result<(), sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                sqlx::query("UPDATE bot_settings SET auto_read = ?, updated_at = CURRENT_TIMESTAMP WHERE id = 1")
                    .bind(if enabled { 1 } else { 0 })
                    .execute(pool)
                    .await?;
            }
            Database::Postgres(pool) => {
                sqlx::query("UPDATE bot_settings SET auto_read = $1, updated_at = CURRENT_TIMESTAMP WHERE id = 1")
                    .bind(enabled)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    async fn set_auto_typing(&self, enabled: bool) -> Result<(), sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                sqlx::query("UPDATE bot_settings SET auto_typing = ?, updated_at = CURRENT_TIMESTAMP WHERE id = 1")
                    .bind(if enabled { 1 } else { 0 })
                    .execute(pool)
                    .await?;
            }
            Database::Postgres(pool) => {
                sqlx::query("UPDATE bot_settings SET auto_typing = $1, updated_at = CURRENT_TIMESTAMP WHERE id = 1")
                    .bind(enabled)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    async fn set_auto_recording(&self, enabled: bool) -> Result<(), sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                sqlx::query("UPDATE bot_settings SET auto_recording = ?, updated_at = CURRENT_TIMESTAMP WHERE id = 1")
                    .bind(if enabled { 1 } else { 0 })
                    .execute(pool)
                    .await?;
            }
            Database::Postgres(pool) => {
                sqlx::query("UPDATE bot_settings SET auto_recording = $1, updated_at = CURRENT_TIMESTAMP WHERE id = 1")
                    .bind(enabled)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    async fn set_reject_call(&self, enabled: bool) -> Result<(), sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                sqlx::query("UPDATE bot_settings SET reject_call = ?, updated_at = CURRENT_TIMESTAMP WHERE id = 1")
                    .bind(if enabled { 1 } else { 0 })
                    .execute(pool)
                    .await?;
            }
            Database::Postgres(pool) => {
                sqlx::query("UPDATE bot_settings SET reject_call = $1, updated_at = CURRENT_TIMESTAMP WHERE id = 1")
                    .bind(enabled)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    async fn set_always_online(&self, enabled: bool) -> Result<(), sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                sqlx::query("UPDATE bot_settings SET always_online = ?, updated_at = CURRENT_TIMESTAMP WHERE id = 1")
                    .bind(if enabled { 1 } else { 0 })
                    .execute(pool)
                    .await?;
            }
            Database::Postgres(pool) => {
                sqlx::query("UPDATE bot_settings SET always_online = $1, updated_at = CURRENT_TIMESTAMP WHERE id = 1")
                    .bind(enabled)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl SessionStore for Database {
    async fn get_session(&self, id: &str) -> Result<Option<SessionModel>, sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                #[derive(sqlx::FromRow)]
                struct SqliteSessionRow {
                    id: String,
                    name: String,
                    category: String,
                    phone_number: Option<String>,
                    prefix: Option<String>,
                    bot_name: Option<String>,
                    auto_start: i64,
                    is_active: i64,
                    created_at: String,
                    updated_at: String,
                }

                let row = sqlx::query_as::<_, SqliteSessionRow>(
                    "SELECT id, name, category, phone_number, prefix, bot_name, auto_start, is_active, datetime(created_at) as created_at, datetime(updated_at) as updated_at FROM bot_sessions WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?;

                Ok(row.map(|r| SessionModel {
                    id: r.id,
                    name: r.name,
                    category: r.category,
                    phone_number: r.phone_number,
                    prefix: r.prefix,
                    bot_name: r.bot_name,
                    auto_start: r.auto_start != 0,
                    is_active: r.is_active != 0,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                }))
            }
            Database::Postgres(pool) => {
                let row = sqlx::query_as::<_, (
                    String,
                    String,
                    String,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    bool,
                    bool,
                    chrono::DateTime<chrono::Utc>,
                    chrono::DateTime<chrono::Utc>,
                )>(
                    "SELECT id, name, category, phone_number, prefix, bot_name, auto_start, is_active, created_at, updated_at FROM bot_sessions WHERE id = $1",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?;

                Ok(row.map(|r| SessionModel {
                    id: r.0,
                    name: r.1,
                    category: r.2,
                    phone_number: r.3,
                    prefix: r.4,
                    bot_name: r.5,
                    auto_start: r.6,
                    is_active: r.7,
                    created_at: r.8.to_rfc3339(),
                    updated_at: r.9.to_rfc3339(),
                }))
            }
        }
    }

    async fn list_sessions(&self) -> Result<Vec<SessionModel>, sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                #[derive(sqlx::FromRow)]
                struct SqliteSessionRow {
                    id: String,
                    name: String,
                    category: String,
                    phone_number: Option<String>,
                    prefix: Option<String>,
                    bot_name: Option<String>,
                    auto_start: i64,
                    is_active: i64,
                    created_at: String,
                    updated_at: String,
                }

                let rows = sqlx::query_as::<_, SqliteSessionRow>(
                    "SELECT id, name, category, phone_number, prefix, bot_name, auto_start, is_active, datetime(created_at) as created_at, datetime(updated_at) as updated_at FROM bot_sessions ORDER BY created_at ASC",
                )
                .fetch_all(pool)
                .await?;

                Ok(rows
                    .into_iter()
                    .map(|r| SessionModel {
                        id: r.id,
                        name: r.name,
                        category: r.category,
                        phone_number: r.phone_number,
                        prefix: r.prefix,
                        bot_name: r.bot_name,
                        auto_start: r.auto_start != 0,
                        is_active: r.is_active != 0,
                        created_at: r.created_at,
                        updated_at: r.updated_at,
                    })
                    .collect())
            }
            Database::Postgres(pool) => {
                let rows = sqlx::query_as::<_, (
                    String,
                    String,
                    String,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    bool,
                    bool,
                    chrono::DateTime<chrono::Utc>,
                    chrono::DateTime<chrono::Utc>,
                )>(
                    "SELECT id, name, category, phone_number, prefix, bot_name, auto_start, is_active, created_at, updated_at FROM bot_sessions ORDER BY created_at ASC",
                )
                .fetch_all(pool)
                .await?;

                Ok(rows
                    .into_iter()
                    .map(|r| SessionModel {
                        id: r.0,
                        name: r.1,
                        category: r.2,
                        phone_number: r.3,
                        prefix: r.4,
                        bot_name: r.5,
                        auto_start: r.6,
                        is_active: r.7,
                        created_at: r.8.to_rfc3339(),
                        updated_at: r.9.to_rfc3339(),
                    })
                    .collect())
            }
        }
    }

    async fn list_sessions_by_category(&self, category: &str) -> Result<Vec<SessionModel>, sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                #[derive(sqlx::FromRow)]
                struct SqliteSessionRow {
                    id: String,
                    name: String,
                    category: String,
                    phone_number: Option<String>,
                    prefix: Option<String>,
                    bot_name: Option<String>,
                    auto_start: i64,
                    is_active: i64,
                    created_at: String,
                    updated_at: String,
                }

                let rows = sqlx::query_as::<_, SqliteSessionRow>(
                    "SELECT id, name, category, phone_number, prefix, bot_name, auto_start, is_active, datetime(created_at) as created_at, datetime(updated_at) as updated_at FROM bot_sessions WHERE category = ? ORDER BY created_at ASC",
                )
                .bind(category)
                .fetch_all(pool)
                .await?;

                Ok(rows
                    .into_iter()
                    .map(|r| SessionModel {
                        id: r.id,
                        name: r.name,
                        category: r.category,
                        phone_number: r.phone_number,
                        prefix: r.prefix,
                        bot_name: r.bot_name,
                        auto_start: r.auto_start != 0,
                        is_active: r.is_active != 0,
                        created_at: r.created_at,
                        updated_at: r.updated_at,
                    })
                    .collect())
            }
            Database::Postgres(pool) => {
                let rows = sqlx::query_as::<_, (
                    String,
                    String,
                    String,
                    Option<String>,
                    Option<String>,
                    Option<String>,
                    bool,
                    bool,
                    chrono::DateTime<chrono::Utc>,
                    chrono::DateTime<chrono::Utc>,
                )>(
                    "SELECT id, name, category, phone_number, prefix, bot_name, auto_start, is_active, created_at, updated_at FROM bot_sessions WHERE category = $1 ORDER BY created_at ASC",
                )
                .bind(category)
                .fetch_all(pool)
                .await?;

                Ok(rows
                    .into_iter()
                    .map(|r| SessionModel {
                        id: r.0,
                        name: r.1,
                        category: r.2,
                        phone_number: r.3,
                        prefix: r.4,
                        bot_name: r.5,
                        auto_start: r.6,
                        is_active: r.7,
                        created_at: r.8.to_rfc3339(),
                        updated_at: r.9.to_rfc3339(),
                    })
                    .collect())
            }
        }
    }

    async fn list_categories(&self) -> Result<Vec<String>, sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                #[derive(sqlx::FromRow)]
                struct CategoryRow {
                    category: String,
                }
                let rows = sqlx::query_as::<_, CategoryRow>(
                    "SELECT DISTINCT category FROM bot_sessions ORDER BY category ASC",
                )
                .fetch_all(pool)
                .await?;

                Ok(rows.into_iter().map(|r| r.category).collect())
            }
            Database::Postgres(pool) => {
                let rows = sqlx::query_as::<_, (String,)>(
                    "SELECT DISTINCT category FROM bot_sessions ORDER BY category ASC",
                )
                .fetch_all(pool)
                .await?;

                Ok(rows.into_iter().map(|r| r.0).collect())
            }
        }
    }

    async fn save_session(&self, session: &SessionModel) -> Result<(), sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                sqlx::query(
                    r#"INSERT INTO bot_sessions (id, name, category, phone_number, prefix, bot_name, auto_start, is_active, updated_at)
                       VALUES (?, ?, ?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
                       ON CONFLICT(id) DO UPDATE SET
                           name = excluded.name,
                           category = excluded.category,
                           phone_number = excluded.phone_number,
                           prefix = excluded.prefix,
                           bot_name = excluded.bot_name,
                           auto_start = excluded.auto_start,
                           is_active = excluded.is_active,
                           updated_at = CURRENT_TIMESTAMP"#,
                )
                .bind(&session.id)
                .bind(&session.name)
                .bind(&session.category)
                .bind(&session.phone_number)
                .bind(&session.prefix)
                .bind(&session.bot_name)
                .bind(if session.auto_start { 1 } else { 0 })
                .bind(if session.is_active { 1 } else { 0 })
                .execute(pool)
                .await?;
            }
            Database::Postgres(pool) => {
                sqlx::query(
                    r#"INSERT INTO bot_sessions (id, name, category, phone_number, prefix, bot_name, auto_start, is_active, updated_at)
                       VALUES ($1, $2, $3, $4, $5, $6, $7, $8, CURRENT_TIMESTAMP)
                       ON CONFLICT(id) DO UPDATE SET
                           name = EXCLUDED.name,
                           category = EXCLUDED.category,
                           phone_number = EXCLUDED.phone_number,
                           prefix = EXCLUDED.prefix,
                           bot_name = EXCLUDED.bot_name,
                           auto_start = EXCLUDED.auto_start,
                           is_active = EXCLUDED.is_active,
                           updated_at = CURRENT_TIMESTAMP"#,
                )
                .bind(&session.id)
                .bind(&session.name)
                .bind(&session.category)
                .bind(&session.phone_number)
                .bind(&session.prefix)
                .bind(&session.bot_name)
                .bind(session.auto_start)
                .bind(session.is_active)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    async fn set_session_active(&self, id: &str, is_active: bool) -> Result<(), sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                sqlx::query("UPDATE bot_sessions SET is_active = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(if is_active { 1 } else { 0 })
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            Database::Postgres(pool) => {
                sqlx::query("UPDATE bot_sessions SET is_active = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
                    .bind(is_active)
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    async fn set_session_phone(&self, id: &str, phone: Option<&str>) -> Result<(), sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                sqlx::query("UPDATE bot_sessions SET phone_number = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(phone)
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            Database::Postgres(pool) => {
                sqlx::query("UPDATE bot_sessions SET phone_number = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
                    .bind(phone)
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    async fn set_session_category(&self, id: &str, category: &str) -> Result<(), sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                sqlx::query("UPDATE bot_sessions SET category = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(category)
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            Database::Postgres(pool) => {
                sqlx::query("UPDATE bot_sessions SET category = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
                    .bind(category)
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    async fn set_session_prefix(&self, id: &str, prefix: Option<&str>) -> Result<(), sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                sqlx::query("UPDATE bot_sessions SET prefix = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(prefix)
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            Database::Postgres(pool) => {
                sqlx::query("UPDATE bot_sessions SET prefix = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
                    .bind(prefix)
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    async fn set_session_bot_name(&self, id: &str, bot_name: Option<&str>) -> Result<(), sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                sqlx::query("UPDATE bot_sessions SET bot_name = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(bot_name)
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            Database::Postgres(pool) => {
                sqlx::query("UPDATE bot_sessions SET bot_name = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
                    .bind(bot_name)
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    async fn set_session_auto_start(&self, id: &str, auto_start: bool) -> Result<(), sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                sqlx::query("UPDATE bot_sessions SET auto_start = ?, updated_at = CURRENT_TIMESTAMP WHERE id = ?")
                    .bind(if auto_start { 1 } else { 0 })
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            Database::Postgres(pool) => {
                sqlx::query("UPDATE bot_sessions SET auto_start = $1, updated_at = CURRENT_TIMESTAMP WHERE id = $2")
                    .bind(auto_start)
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }

    async fn delete_session(&self, id: &str) -> Result<(), sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                sqlx::query("DELETE FROM bot_sessions WHERE id = ?")
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
            Database::Postgres(pool) => {
                sqlx::query("DELETE FROM bot_sessions WHERE id = $1")
                    .bind(id)
                    .execute(pool)
                    .await?;
            }
        }
        Ok(())
    }
}

#[async_trait]
impl MessageStore for Database {
    async fn save_message_bytes(
        &self,
        id: &str,
        chat: &str,
        sender: &str,
        bytes: &[u8],
    ) -> Result<(), sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                sqlx::query(
                    r#"INSERT INTO bot_messages (id, chat, sender, raw_message)
                       VALUES (?, ?, ?, ?)
                       ON CONFLICT(id) DO UPDATE SET raw_message = excluded.raw_message"#,
                )
                .bind(id)
                .bind(chat)
                .bind(sender)
                .bind(bytes)
                .execute(pool)
                .await?;
            }
            Database::Postgres(pool) => {
                sqlx::query(
                    r#"INSERT INTO bot_messages (id, chat, sender, raw_message, created_at)
                       VALUES ($1, $2, $3, $4, CURRENT_TIMESTAMP)
                       ON CONFLICT(id) DO UPDATE SET raw_message = EXCLUDED.raw_message"#,
                )
                .bind(id)
                .bind(chat)
                .bind(sender)
                .bind(bytes)
                .execute(pool)
                .await?;
            }
        }
        Ok(())
    }

    async fn get_message_bytes(&self, id: &str) -> Result<Option<Vec<u8>>, sqlx::Error> {
        match self {
            Database::Sqlite(pool) => {
                #[derive(sqlx::FromRow)]
                struct MsgRow {
                    raw_message: Vec<u8>,
                }

                let row = sqlx::query_as::<_, MsgRow>(
                    "SELECT raw_message FROM bot_messages WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?;

                if let Some(r) = row {
                    return Ok(Some(r.raw_message));
                }

                #[derive(sqlx::FromRow)]
                struct InboundRow {
                    message: Vec<u8>,
                }
                let in_row = sqlx::query_as::<_, InboundRow>(
                    "SELECT message FROM pending_inbound_messages WHERE id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?;

                if let Some(r) = in_row {
                    return Ok(Some(r.message));
                }

                #[derive(sqlx::FromRow)]
                struct SentRow {
                    payload: Vec<u8>,
                }
                let sent_row = sqlx::query_as::<_, SentRow>(
                    "SELECT payload FROM sent_messages WHERE message_id = ?",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?;

                if let Some(r) = sent_row {
                    return Ok(Some(r.payload));
                }

                Ok(None)
            }
            Database::Postgres(pool) => {
                let row = sqlx::query_as::<_, (Vec<u8>,)>(
                    "SELECT raw_message FROM bot_messages WHERE id = $1",
                )
                .bind(id)
                .fetch_optional(pool)
                .await?;

                Ok(row.map(|r| r.0))
            }
        }
    }
}
