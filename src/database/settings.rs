use async_trait::async_trait;
use sea_orm::DbErr;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BotSettings {
    pub id: i64,
    pub prefix: String,
    pub bot_name: String,
    pub work_type: String,
    pub auto_read: bool,
    pub auto_typing: bool,
    pub auto_recording: bool,
    pub reject_call: bool,
    pub always_online: bool,
}

#[allow(dead_code)]
#[async_trait]
pub trait SettingsStore: Send + Sync {
    async fn get_settings(&self) -> Result<BotSettings, DbErr>;
    async fn set_prefix(&self, prefix: &str) -> Result<(), DbErr>;
    async fn set_bot_name(&self, name: &str) -> Result<(), DbErr>;
    async fn set_work_type(&self, work_type: &str) -> Result<(), DbErr>;
    async fn set_auto_read(&self, enabled: bool) -> Result<(), DbErr>;
    async fn set_auto_typing(&self, enabled: bool) -> Result<(), DbErr>;
    async fn set_auto_recording(&self, enabled: bool) -> Result<(), DbErr>;
    async fn set_reject_call(&self, enabled: bool) -> Result<(), DbErr>;
    async fn set_always_online(&self, enabled: bool) -> Result<(), DbErr>;
}
