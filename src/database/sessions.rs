use async_trait::async_trait;

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
    async fn get_session(&self, id: &str) -> Result<Option<SessionModel>, sqlx::Error>;
    async fn list_sessions(&self) -> Result<Vec<SessionModel>, sqlx::Error>;
    async fn list_sessions_by_category(&self, category: &str) -> Result<Vec<SessionModel>, sqlx::Error>;
    async fn list_categories(&self) -> Result<Vec<String>, sqlx::Error>;
    async fn save_session(&self, session: &SessionModel) -> Result<(), sqlx::Error>;
    async fn set_session_active(&self, id: &str, is_active: bool) -> Result<(), sqlx::Error>;
    async fn set_session_phone(&self, id: &str, phone: Option<&str>) -> Result<(), sqlx::Error>;
    async fn set_session_category(&self, id: &str, category: &str) -> Result<(), sqlx::Error>;
    async fn set_session_prefix(&self, id: &str, prefix: Option<&str>) -> Result<(), sqlx::Error>;
    async fn set_session_bot_name(&self, id: &str, bot_name: Option<&str>) -> Result<(), sqlx::Error>;
    async fn set_session_auto_start(&self, id: &str, auto_start: bool) -> Result<(), sqlx::Error>;
    async fn delete_session(&self, id: &str) -> Result<(), sqlx::Error>;
}
