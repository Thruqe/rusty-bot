use super::session::{Session, SessionSummary};
use crate::database::{Database, SessionModel, SessionStore};
use crate::plugins::CommandManager;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

#[allow(dead_code)]
pub struct SessionManager {
    pub db: Arc<Database>,
    pub command_manager: Arc<CommandManager>,
    pub sessions_dir: PathBuf,
    pub sessions: Arc<RwLock<HashMap<String, Arc<Session>>>>,
}

#[allow(dead_code)]
impl SessionManager {
    pub fn new(
        db: Arc<Database>,
        command_manager: Arc<CommandManager>,
        sessions_dir: impl Into<PathBuf>,
    ) -> Arc<Self> {
        Arc::new(Self {
            db,
            command_manager,
            sessions_dir: sessions_dir.into(),
            sessions: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    fn resolve_db_path(&self, session_id: &str) -> PathBuf {
        if session_id == "default" {
            let root_db = Path::new("whatsapp.db");
            if root_db.exists() {
                return root_db.to_path_buf();
            }
        }
        self.sessions_dir.join(format!("{}.db", session_id))
    }

    pub async fn init(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tokio::fs::create_dir_all(&self.sessions_dir).await?;

        let mut models = self.db.list_sessions().await?;
        if models.is_empty() {
            let default_model = SessionModel {
                id: "default".to_string(),
                name: "Default Session".to_string(),
                category: "default".to_string(),
                phone_number: None,
                prefix: None,
                bot_name: None,
                auto_start: true,
                is_active: false,
                created_at: chrono::Utc::now().to_rfc3339(),
                updated_at: chrono::Utc::now().to_rfc3339(),
            };
            self.db.save_session(&default_model).await?;
            models.push(default_model);
        }

        let mut map = self.sessions.write().await;
        for model in models {
            let db_path = self.resolve_db_path(&model.id);
            let session = Arc::new(Session::new(
                model.clone(),
                db_path,
                self.command_manager.clone(),
                self.db.clone(),
            ));

            let auto_start = model.auto_start;
            let session_clone = session.clone();
            map.insert(model.id.clone(), session);

            if auto_start {
                tokio::spawn(async move {
                    if let Err(e) = session_clone.start().await {
                        eprintln!("[session:{}] Failed to start: {e}", session_clone.id().await);
                    }
                });
            }
        }

        Ok(())
    }

    pub async fn create_session(
        &self,
        id: &str,
        name: &str,
        category: &str,
        prefix: Option<&str>,
        bot_name: Option<&str>,
        auto_start: bool,
    ) -> Result<Arc<Session>, Box<dyn std::error::Error + Send + Sync>> {
        let clean_id = id.trim().to_lowercase();
        if clean_id.is_empty()
            || !clean_id
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err("Invalid session ID. Use alphanumeric, dash, or underscore characters.".into());
        }

        {
            let map = self.sessions.read().await;
            if map.contains_key(&clean_id) {
                return Err(format!("Session with ID '{clean_id}' already exists").into());
            }
        }

        let model = SessionModel {
            id: clean_id.clone(),
            name: name.to_string(),
            category: if category.trim().is_empty() {
                "default".to_string()
            } else {
                category.trim().to_lowercase()
            },
            phone_number: None,
            prefix: prefix.map(|s| s.to_string()),
            bot_name: bot_name.map(|s| s.to_string()),
            auto_start,
            is_active: false,
            created_at: chrono::Utc::now().to_rfc3339(),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        self.db.save_session(&model).await?;

        let db_path = self.resolve_db_path(&clean_id);
        let session = Arc::new(Session::new(
            model,
            db_path,
            self.command_manager.clone(),
            self.db.clone(),
        ));

        self.sessions
            .write()
            .await
            .insert(clean_id.clone(), session.clone());

        if auto_start {
            let session_clone = session.clone();
            tokio::spawn(async move {
                if let Err(e) = session_clone.start().await {
                    eprintln!("[session:{}] Failed to auto-start: {e}", clean_id);
                }
            });
        }

        Ok(session)
    }

    pub async fn start_session(
        &self,
        id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let session = self
            .get_session(id)
            .await
            .ok_or_else(|| format!("Session '{id}' not found"))?;
        session.start().await
    }

    pub async fn stop_session(
        &self,
        id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let session = self
            .get_session(id)
            .await
            .ok_or_else(|| format!("Session '{id}' not found"))?;
        session.stop().await
    }

    pub async fn restart_session(
        &self,
        id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let session = self
            .get_session(id)
            .await
            .ok_or_else(|| format!("Session '{id}' not found"))?;
        session.restart().await
    }

    pub async fn delete_session(
        &self,
        id: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let session = self
            .sessions
            .write()
            .await
            .remove(id)
            .ok_or_else(|| format!("Session '{id}' not found"))?;

        let _ = session.stop().await;
        self.db.delete_session(id).await?;

        let db_path = self.resolve_db_path(id);
        if db_path.exists() && id != "default" {
            let _ = tokio::fs::remove_file(&db_path).await;
            let shm = db_path.with_extension("db-shm");
            let wal = db_path.with_extension("db-wal");
            let _ = tokio::fs::remove_file(shm).await;
            let _ = tokio::fs::remove_file(wal).await;
        }

        Ok(())
    }

    pub async fn get_session(&self, id: &str) -> Option<Arc<Session>> {
        self.sessions.read().await.get(id).cloned()
    }

    pub async fn list_sessions(&self) -> Vec<SessionSummary> {
        let map = self.sessions.read().await;
        let mut list = Vec::new();
        for session in map.values() {
            list.push(session.summary().await);
        }
        list.sort_by(|a, b| a.id.cmp(&b.id));
        list
    }

    pub async fn list_by_category(&self, category: &str) -> Vec<SessionSummary> {
        let map = self.sessions.read().await;
        let mut list = Vec::new();
        let target_cat = category.trim().to_lowercase();
        for session in map.values() {
            let summary = session.summary().await;
            if summary.category.to_lowercase() == target_cat {
                list.push(summary);
            }
        }
        list.sort_by(|a, b| a.id.cmp(&b.id));
        list
    }

    pub async fn list_categories(&self) -> Vec<String> {
        let map = self.sessions.read().await;
        let mut set = std::collections::BTreeSet::new();
        for session in map.values() {
            set.insert(session.category().await);
        }
        set.into_iter().collect()
    }

    pub async fn update_session_category(
        &self,
        id: &str,
        category: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let session = self
            .get_session(id)
            .await
            .ok_or_else(|| format!("Session '{id}' not found"))?;
        let cat = category.trim().to_lowercase();
        self.db.set_session_category(id, &cat).await?;
        session.model.write().await.category = cat;
        Ok(())
    }

    pub async fn update_session_prefix(
        &self,
        id: &str,
        prefix: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let session = self
            .get_session(id)
            .await
            .ok_or_else(|| format!("Session '{id}' not found"))?;
        self.db.set_session_prefix(id, prefix).await?;
        session.model.write().await.prefix = prefix.map(|s| s.to_string());
        Ok(())
    }

    pub async fn update_session_bot_name(
        &self,
        id: &str,
        bot_name: Option<&str>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let session = self
            .get_session(id)
            .await
            .ok_or_else(|| format!("Session '{id}' not found"))?;
        self.db.set_session_bot_name(id, bot_name).await?;
        session.model.write().await.bot_name = bot_name.map(|s| s.to_string());
        Ok(())
    }

    pub async fn update_session_auto_start(
        &self,
        id: &str,
        auto_start: bool,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let session = self
            .get_session(id)
            .await
            .ok_or_else(|| format!("Session '{id}' not found"))?;
        self.db.set_session_auto_start(id, auto_start).await?;
        session.model.write().await.auto_start = auto_start;
        Ok(())
    }

    pub async fn get_qr(&self, id: &str) -> Option<(String, String)> {
        if let Some(session) = self.get_session(id).await {
            session.get_qr().await
        } else {
            None
        }
    }

    pub async fn shutdown_all(&self) {
        let map = self.sessions.read().await;
        println!("[session-manager] Shutting down all active sessions...");
        for (id, session) in map.iter() {
            if session.is_running().await {
                println!("[session:{}] Gracefully shutting down...", id);
                let _ = session.stop().await;
            }
        }
    }
}
