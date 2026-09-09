use async_trait::async_trait;
use sea_orm::DbErr;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionModel {
    pub id: String,
    pub name: String,
    pub category: String,
    pub phone_number: Option<String>,
    pub prefix: Option<String>,
    pub bot_name: Option<String>,
    pub auto_start: bool,
    pub is_active: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[allow(dead_code)]
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn get_session(&self, id: &str) -> Result<Option<SessionModel>, DbErr>;
    async fn list_sessions(&self) -> Result<Vec<SessionModel>, DbErr>;
    async fn list_sessions_by_category(&self, category: &str) -> Result<Vec<SessionModel>, DbErr>;
    async fn list_categories(&self) -> Result<Vec<String>, DbErr>;
    async fn save_session(&self, session: &SessionModel) -> Result<(), DbErr>;
    async fn set_session_active(&self, id: &str, is_active: bool) -> Result<(), DbErr>;
    async fn set_session_phone(&self, id: &str, phone: Option<&str>) -> Result<(), DbErr>;
    async fn set_session_category(&self, id: &str, category: &str) -> Result<(), DbErr>;
    async fn set_session_prefix(&self, id: &str, prefix: Option<&str>) -> Result<(), DbErr>;
    async fn set_session_bot_name(&self, id: &str, bot_name: Option<&str>) -> Result<(), DbErr>;
    async fn set_session_auto_start(&self, id: &str, auto_start: bool) -> Result<(), DbErr>;
    async fn delete_session(&self, id: &str) -> Result<(), DbErr>;
}
