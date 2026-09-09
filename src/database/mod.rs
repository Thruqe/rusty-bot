pub mod entities;
pub mod messages;
pub mod sessions;
pub mod settings;

use async_trait::async_trait;
use chrono::Utc;
use entities::{bot_messages, bot_sessions, bot_settings};
pub use messages::MessageStore;
use sea_orm::ConnectionTrait;
use sea_orm::sea_query::{Expr, ExprTrait, Index, OnConflict, Table};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectOptions, Database as SeaDatabase, DatabaseConnection,
    DbErr, EntityTrait, IntoActiveModel, QueryFilter, QueryOrder, QuerySelect, Set,
};
pub use sessions::{SessionModel, SessionStore};
pub use settings::{BotSettings, SettingsStore};
use std::sync::Arc;

#[derive(Clone)]
pub enum Database {
    Sqlite(DatabaseConnection),
    Postgres(DatabaseConnection),
}

impl Database {
    pub async fn new_sqlite(database_url: &str) -> Result<Arc<Self>, DbErr> {
        let url = if database_url.starts_with("sqlite:") {
            database_url.to_string()
        } else {
            format!("sqlite:{}?mode=rwc", database_url)
        };
        let mut options = ConnectOptions::new(url);
        let in_memory = database_url.contains(":memory:");
        options
            .max_connections(if in_memory { 1 } else { 2 })
            .min_connections(1);
        let connection = SeaDatabase::connect(options).await?;
        let db = Arc::new(Database::Sqlite(connection));
        db.init_tables().await?;
        Ok(db)
    }

    pub async fn new_postgres(database_url: &str) -> Result<Arc<Self>, DbErr> {
        let mut options = ConnectOptions::new(database_url);
        options.max_connections(2).min_connections(1);
        let connection = SeaDatabase::connect(options).await?;
        let db = Arc::new(Database::Postgres(connection));
        db.init_tables().await?;
        Ok(db)
    }

    fn connection(&self) -> &DatabaseConnection {
        match self {
            Self::Sqlite(connection) | Self::Postgres(connection) => connection,
        }
    }

    pub async fn init_tables(&self) -> Result<(), DbErr> {
        let db = self.connection();
        let settings = Table::create()
            .table(bot_settings::Entity)
            .if_not_exists()
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_settings::Column::Id)
                    .small_integer()
                    .not_null()
                    .primary_key()
                    .check(Expr::col(bot_settings::Column::Id).eq(Expr::val(1))),
            )
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_settings::Column::Prefix)
                    .string_len(10)
                    .not_null()
                    .default("."),
            )
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_settings::Column::BotName)
                    .string_len(100)
                    .not_null()
                    .default("WaBot"),
            )
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_settings::Column::WorkType)
                    .string_len(20)
                    .not_null()
                    .default("public"),
            )
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_settings::Column::AutoRead)
                    .boolean()
                    .not_null()
                    .default(false),
            )
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_settings::Column::AutoTyping)
                    .boolean()
                    .not_null()
                    .default(false),
            )
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_settings::Column::AutoRecording)
                    .boolean()
                    .not_null()
                    .default(false),
            )
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_settings::Column::RejectCall)
                    .boolean()
                    .not_null()
                    .default(false),
            )
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_settings::Column::AlwaysOnline)
                    .boolean()
                    .not_null()
                    .default(true),
            )
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_settings::Column::CreatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_settings::Column::UpdatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .to_owned();
        db.execute(&settings).await?;

        let sessions = Table::create()
            .table(bot_sessions::Entity)
            .if_not_exists()
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_sessions::Column::Id)
                    .string_len(64)
                    .not_null()
                    .primary_key(),
            )
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_sessions::Column::Name)
                    .string_len(100)
                    .not_null(),
            )
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_sessions::Column::Category)
                    .string_len(50)
                    .not_null()
                    .default("default"),
            )
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_sessions::Column::PhoneNumber)
                    .string_len(32),
            )
            .col(sea_orm::sea_query::ColumnDef::new(bot_sessions::Column::Prefix).string_len(10))
            .col(sea_orm::sea_query::ColumnDef::new(bot_sessions::Column::BotName).string_len(100))
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_sessions::Column::AutoStart)
                    .boolean()
                    .not_null()
                    .default(true),
            )
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_sessions::Column::IsActive)
                    .boolean()
                    .not_null()
                    .default(false),
            )
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_sessions::Column::CreatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_sessions::Column::UpdatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .to_owned();
        db.execute(&sessions).await?;

        let messages = Table::create()
            .table(bot_messages::Entity)
            .if_not_exists()
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_messages::Column::Id)
                    .string_len(128)
                    .not_null()
                    .primary_key(),
            )
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_messages::Column::Chat)
                    .string_len(128)
                    .not_null(),
            )
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_messages::Column::Sender)
                    .string_len(128)
                    .not_null(),
            )
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_messages::Column::RawMessage)
                    .binary()
                    .not_null(),
            )
            .col(
                sea_orm::sea_query::ColumnDef::new(bot_messages::Column::CreatedAt)
                    .timestamp_with_time_zone()
                    .not_null()
                    .default(Expr::current_timestamp()),
            )
            .to_owned();
        db.execute(&messages).await?;

        let message_index = Index::create()
            .name("idx_bot_messages_chat")
            .table(bot_messages::Entity)
            .col(bot_messages::Column::Chat)
            .if_not_exists()
            .to_owned();
        db.execute(&message_index).await?;

        bot_settings::Entity::insert(bot_settings::ActiveModel {
            id: Set(1),
            prefix: Set(".".to_owned()),
            bot_name: Set("WaBot".to_owned()),
            work_type: Set("public".to_owned()),
            auto_read: Set(false),
            auto_typing: Set(false),
            auto_recording: Set(false),
            reject_call: Set(false),
            always_online: Set(true),
            created_at: Default::default(),
            updated_at: Default::default(),
        })
        .on_conflict(
            OnConflict::column(bot_settings::Column::Id)
                .do_nothing()
                .to_owned(),
        )
        .exec(db)
        .await?;

        bot_sessions::Entity::insert(bot_sessions::ActiveModel {
            id: Set("default".to_owned()),
            name: Set("Default Session".to_owned()),
            category: Set("default".to_owned()),
            phone_number: Set(None),
            prefix: Set(None),
            bot_name: Set(None),
            auto_start: Set(true),
            is_active: Set(false),
            created_at: Default::default(),
            updated_at: Default::default(),
        })
        .on_conflict(
            OnConflict::column(bot_sessions::Column::Id)
                .do_nothing()
                .to_owned(),
        )
        .exec(db)
        .await?;

        Ok(())
    }

    async fn update_settings(
        &self,
        update: impl FnOnce(&mut bot_settings::ActiveModel),
    ) -> Result<(), DbErr> {
        let Some(model) = bot_settings::Entity::find_by_id(1_i16)
            .one(self.connection())
            .await?
        else {
            return Ok(());
        };
        let mut active = model.into_active_model();
        update(&mut active);
        active.updated_at = Set(Utc::now());
        active.update(self.connection()).await?;
        Ok(())
    }

    async fn update_session(
        &self,
        id: &str,
        update: impl FnOnce(&mut bot_sessions::ActiveModel),
    ) -> Result<(), DbErr> {
        let Some(model) = bot_sessions::Entity::find_by_id(id)
            .one(self.connection())
            .await?
        else {
            return Ok(());
        };
        let mut active = model.into_active_model();
        update(&mut active);
        active.updated_at = Set(Utc::now());
        active.update(self.connection()).await?;
        Ok(())
    }
}

fn settings_model(model: bot_settings::Model) -> BotSettings {
    BotSettings {
        id: model.id as i64,
        prefix: model.prefix,
        bot_name: model.bot_name,
        work_type: model.work_type,
        auto_read: model.auto_read,
        auto_typing: model.auto_typing,
        auto_recording: model.auto_recording,
        reject_call: model.reject_call,
        always_online: model.always_online,
    }
}

fn session_model(model: bot_sessions::Model) -> SessionModel {
    SessionModel {
        id: model.id,
        name: model.name,
        category: model.category,
        phone_number: model.phone_number,
        prefix: model.prefix,
        bot_name: model.bot_name,
        auto_start: model.auto_start,
        is_active: model.is_active,
        created_at: model.created_at.to_rfc3339(),
        updated_at: model.updated_at.to_rfc3339(),
    }
}

#[async_trait]
impl SettingsStore for Database {
    async fn get_settings(&self) -> Result<BotSettings, DbErr> {
        bot_settings::Entity::find_by_id(1_i16)
            .one(self.connection())
            .await?
            .map(settings_model)
            .ok_or_else(|| DbErr::RecordNotFound("bot_settings id=1".to_owned()))
    }

    async fn set_prefix(&self, prefix: &str) -> Result<(), DbErr> {
        self.update_settings(|model| model.prefix = Set(prefix.to_owned()))
            .await
    }

    async fn set_bot_name(&self, name: &str) -> Result<(), DbErr> {
        self.update_settings(|model| model.bot_name = Set(name.to_owned()))
            .await
    }

    async fn set_work_type(&self, work_type: &str) -> Result<(), DbErr> {
        self.update_settings(|model| model.work_type = Set(work_type.to_owned()))
            .await
    }

    async fn set_auto_read(&self, enabled: bool) -> Result<(), DbErr> {
        self.update_settings(|model| model.auto_read = Set(enabled))
            .await
    }

    async fn set_auto_typing(&self, enabled: bool) -> Result<(), DbErr> {
        self.update_settings(|model| model.auto_typing = Set(enabled))
            .await
    }

    async fn set_auto_recording(&self, enabled: bool) -> Result<(), DbErr> {
        self.update_settings(|model| model.auto_recording = Set(enabled))
            .await
    }

    async fn set_reject_call(&self, enabled: bool) -> Result<(), DbErr> {
        self.update_settings(|model| model.reject_call = Set(enabled))
            .await
    }

    async fn set_always_online(&self, enabled: bool) -> Result<(), DbErr> {
        self.update_settings(|model| model.always_online = Set(enabled))
            .await
    }
}

#[async_trait]
impl SessionStore for Database {
    async fn get_session(&self, id: &str) -> Result<Option<SessionModel>, DbErr> {
        Ok(bot_sessions::Entity::find_by_id(id)
            .one(self.connection())
            .await?
            .map(session_model))
    }

    async fn list_sessions(&self) -> Result<Vec<SessionModel>, DbErr> {
        Ok(bot_sessions::Entity::find()
            .order_by_asc(bot_sessions::Column::CreatedAt)
            .all(self.connection())
            .await?
            .into_iter()
            .map(session_model)
            .collect())
    }

    async fn list_sessions_by_category(&self, category: &str) -> Result<Vec<SessionModel>, DbErr> {
        Ok(bot_sessions::Entity::find()
            .filter(bot_sessions::Column::Category.eq(category))
            .order_by_asc(bot_sessions::Column::CreatedAt)
            .all(self.connection())
            .await?
            .into_iter()
            .map(session_model)
            .collect())
    }

    async fn list_categories(&self) -> Result<Vec<String>, DbErr> {
        Ok(bot_sessions::Entity::find()
            .select_only()
            .column(bot_sessions::Column::Category)
            .distinct()
            .order_by_asc(bot_sessions::Column::Category)
            .into_tuple::<String>()
            .all(self.connection())
            .await?)
    }

    async fn save_session(&self, session: &SessionModel) -> Result<(), DbErr> {
        let active = bot_sessions::ActiveModel {
            id: Set(session.id.clone()),
            name: Set(session.name.clone()),
            category: Set(session.category.clone()),
            phone_number: Set(session.phone_number.clone()),
            prefix: Set(session.prefix.clone()),
            bot_name: Set(session.bot_name.clone()),
            auto_start: Set(session.auto_start),
            is_active: Set(session.is_active),
            created_at: Default::default(),
            updated_at: Set(Utc::now()),
        };
        bot_sessions::Entity::insert(active)
            .on_conflict(
                OnConflict::column(bot_sessions::Column::Id)
                    .update_columns([
                        bot_sessions::Column::Name,
                        bot_sessions::Column::Category,
                        bot_sessions::Column::PhoneNumber,
                        bot_sessions::Column::Prefix,
                        bot_sessions::Column::BotName,
                        bot_sessions::Column::AutoStart,
                        bot_sessions::Column::IsActive,
                        bot_sessions::Column::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .exec(self.connection())
            .await?;
        Ok(())
    }

    async fn set_session_active(&self, id: &str, is_active: bool) -> Result<(), DbErr> {
        self.update_session(id, |model| model.is_active = Set(is_active))
            .await
    }

    async fn set_session_phone(&self, id: &str, phone: Option<&str>) -> Result<(), DbErr> {
        self.update_session(id, |model| {
            model.phone_number = Set(phone.map(str::to_owned))
        })
        .await
    }

    async fn set_session_category(&self, id: &str, category: &str) -> Result<(), DbErr> {
        self.update_session(id, |model| model.category = Set(category.to_owned()))
            .await
    }

    async fn set_session_prefix(&self, id: &str, prefix: Option<&str>) -> Result<(), DbErr> {
        self.update_session(id, |model| model.prefix = Set(prefix.map(str::to_owned)))
            .await
    }

    async fn set_session_bot_name(&self, id: &str, bot_name: Option<&str>) -> Result<(), DbErr> {
        self.update_session(id, |model| {
            model.bot_name = Set(bot_name.map(str::to_owned))
        })
        .await
    }

    async fn set_session_auto_start(&self, id: &str, auto_start: bool) -> Result<(), DbErr> {
        self.update_session(id, |model| model.auto_start = Set(auto_start))
            .await
    }

    async fn delete_session(&self, id: &str) -> Result<(), DbErr> {
        bot_sessions::Entity::delete_by_id(id)
            .exec(self.connection())
            .await?;
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
    ) -> Result<(), DbErr> {
        bot_messages::Entity::insert(bot_messages::ActiveModel {
            id: Set(id.to_owned()),
            chat: Set(chat.to_owned()),
            sender: Set(sender.to_owned()),
            raw_message: Set(bytes.to_vec()),
            created_at: Default::default(),
        })
        .on_conflict(
            OnConflict::column(bot_messages::Column::Id)
                .update_columns([bot_messages::Column::RawMessage])
                .to_owned(),
        )
        .exec(self.connection())
        .await?;
        Ok(())
    }

    async fn get_message_bytes(&self, id: &str) -> Result<Option<Vec<u8>>, DbErr> {
        let Some(message) = bot_messages::Entity::find_by_id(id)
            .one(self.connection())
            .await?
        else {
            if !matches!(self, Self::Sqlite(_)) {
                return Ok(None);
            }

            if let Some(message) = entities::pending_inbound_messages::Entity::find_by_id(id)
                .one(self.connection())
                .await?
            {
                return Ok(Some(message.message));
            }
            if let Some(message) = entities::sent_messages::Entity::find_by_id(id)
                .one(self.connection())
                .await?
            {
                return Ok(Some(message.payload));
            }
            return Ok(None);
        };
        Ok(Some(message.raw_message))
    }
}
