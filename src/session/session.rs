use crate::database::{Database, MessageStore, SessionModel, SessionStore, SettingsStore};
use crate::plugins::CommandManager;
use crate::serialize::SerializedMessage;
use qrcode::render::unicode;
use qrcode::{EcLevel, QrCode};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, RwLock};
use whatsapp_rust::bot::BotHandle;
use whatsapp_rust::prelude::*;
use whatsapp_rust::wacore_binary::JidExt;

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionStatus {
    Stopped,
    Starting,
    QrReady { qr_ascii: String, qr_raw: String },
    Connected { jid: String, phone: String },
    Disconnected { reason: String },
    LoggedOut,
    Failed { error: String },
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct SessionSummary {
    pub id: String,
    pub name: String,
    pub category: String,
    pub phone_number: Option<String>,
    pub prefix: Option<String>,
    pub bot_name: Option<String>,
    pub auto_start: bool,
    pub is_active: bool,
    pub status: SessionStatus,
}

#[allow(dead_code)]
pub struct Session {
    pub model: RwLock<SessionModel>,
    pub status: Arc<RwLock<SessionStatus>>,
    pub db_path: PathBuf,
    pub bot_handle: Arc<Mutex<Option<BotHandle>>>,
    pub client: Arc<RwLock<Option<Arc<whatsapp_rust::Client>>>>,
    pub command_manager: Arc<CommandManager>,
    pub database: Arc<Database>,
}

#[allow(dead_code)]
impl Session {
    pub fn new(
        model: SessionModel,
        db_path: PathBuf,
        command_manager: Arc<CommandManager>,
        database: Arc<Database>,
    ) -> Self {
        Self {
            model: RwLock::new(model),
            status: Arc::new(RwLock::new(SessionStatus::Stopped)),
            db_path,
            bot_handle: Arc::new(Mutex::new(None)),
            client: Arc::new(RwLock::new(None)),
            command_manager,
            database,
        }
    }

    pub async fn id(&self) -> String {
        self.model.read().await.id.clone()
    }

    pub async fn name(&self) -> String {
        self.model.read().await.name.clone()
    }

    pub async fn category(&self) -> String {
        self.model.read().await.category.clone()
    }

    pub async fn prefix(&self) -> Option<String> {
        self.model.read().await.prefix.clone()
    }

    pub async fn bot_name(&self) -> Option<String> {
        self.model.read().await.bot_name.clone()
    }

    pub async fn auto_start(&self) -> bool {
        self.model.read().await.auto_start
    }

    pub async fn status(&self) -> SessionStatus {
        self.status.read().await.clone()
    }

    pub async fn is_running(&self) -> bool {
        self.bot_handle.lock().await.is_some()
    }

    pub async fn summary(&self) -> SessionSummary {
        let model = self.model.read().await.clone();
        let status = self.status.read().await.clone();
        SessionSummary {
            id: model.id,
            name: model.name,
            category: model.category,
            phone_number: model.phone_number,
            prefix: model.prefix,
            bot_name: model.bot_name,
            auto_start: model.auto_start,
            is_active: model.is_active,
            status,
        }
    }

    pub async fn get_qr(&self) -> Option<(String, String)> {
        match &*self.status.read().await {
            SessionStatus::QrReady { qr_ascii, qr_raw } => {
                Some((qr_ascii.clone(), qr_raw.clone()))
            }
            _ => None,
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut handle_guard = self.bot_handle.lock().await;
        if handle_guard.is_some() {
            println!("[session:{}] Already running", self.id().await);
            return Ok(());
        }

        *self.status.write().await = SessionStatus::Starting;
        println!("[session:{}] Starting session...", self.id().await);

        if let Some(parent) = self.db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
        }

        let db_path_str = self.db_path.to_string_lossy().to_string();
        let store = SqliteStore::new(&db_path_str).await?;

        let session_id = self.id().await;
        let session_name = self.name().await;
        let session_category = self.category().await;
        let session_prefix = self.prefix().await;
        let session_bot_name = self.bot_name().await;

        let status_clone = self.status.clone();
        let db_clone = self.database.clone();
        let s_id_for_qr = session_id.clone();
        let s_name_for_qr = session_name.clone();

        let s_id_for_events = session_id.clone();
        let status_for_events = self.status.clone();
        let db_for_events = self.database.clone();

        let manager = self.command_manager.clone();
        let s_id_for_msg = session_id.clone();
        let s_name_for_msg = session_name.clone();
        let s_cat_for_msg = session_category.clone();
        let s_pfx_for_msg = session_prefix.clone();
        let s_bn_for_msg = session_bot_name.clone();
        let db_msg_for_store = self.database.clone();

        let bot = Bot::builder()
            .with_backend(store)
            .on_qr_code(move |code, _timeout| {
                let status = status_clone.clone();
                let sid = s_id_for_qr.clone();
                let sname = s_name_for_qr.clone();
                async move {
                    if let Ok(qr) = QrCode::with_error_correction_level(code.as_bytes(), EcLevel::L)
                    {
                        let image = qr
                            .render::<unicode::Dense1x2>()
                            .dark_color(unicode::Dense1x2::Dark)
                            .light_color(unicode::Dense1x2::Light)
                            .quiet_zone(true)
                            .build();

                        *status.write().await = SessionStatus::QrReady {
                            qr_ascii: image.clone(),
                            qr_raw: code.to_string(),
                        };

                        println!("\n[session:{}] qr code for {}:\n{}\n", sid, sname, image);
                    }
                }
            })
            .on_event(move |event, client| {
                let sid = s_id_for_events.clone();
                let status = status_for_events.clone();
                let db = db_for_events.clone();
                async move {
                    match event.as_ref() {
                        Event::Connected(_) => {
                            let snapshot = client.persistence_manager().get_device_snapshot();
                            let (jid_str, phone) = if let Some(ref pn) = snapshot.pn {
                                (pn.to_string(), pn.user().to_string())
                            } else {
                                ("connected".to_string(), "unknown".to_string())
                            };
                            *status.write().await = SessionStatus::Connected {
                                jid: jid_str.clone(),
                                phone: phone.clone(),
                            };
                            println!("[session:{}] Connected as {} ({})", sid, phone, jid_str);
                            if phone != "unknown" {
                                let _ = db.set_session_phone(&sid, Some(&phone)).await;
                            }
                            let _ = db.set_session_active(&sid, true).await;

                            if let Ok(settings) = db.get_settings().await {
                                if settings.always_online {
                                    let _ = client.presence().set_available().await;
                                }
                            }
                        }
                        Event::Disconnected(disc) => {
                            let reason = format!("{:?}", disc.reason);
                            *status.write().await = SessionStatus::Disconnected {
                                reason: reason.clone(),
                            };
                            println!("[session:{}] Disconnected: {}", sid, reason);
                        }
                        Event::LoggedOut(_) => {
                            *status.write().await = SessionStatus::LoggedOut;
                            println!("[session:{}] Logged out", sid);
                            let _ = db.set_session_active(&sid, false).await;
                        }
                        _ => {}
                    }
                }
            })
            .on_message({
                let db_settings = db_clone.clone();
                move |ctx| {
                    let manager = manager.clone();
                    let s_id = s_id_for_msg.clone();
                    let s_name = s_name_for_msg.clone();
                    let s_cat = s_cat_for_msg.clone();
                    let s_pfx = s_pfx_for_msg.clone();
                    let s_bn = s_bn_for_msg.clone();
                    let db_settings = db_settings.clone();
                    let db_store = db_msg_for_store.clone();

                    async move {
                        let msg_id = ctx.info.id.to_string();
                        let chat_str = ctx.info.source.chat.to_string();
                        let sender_str = ctx.info.source.sender.to_string();

                        let _ = db_store
                            .save_wa_message(&msg_id, &chat_str, &sender_str, &ctx.message)
                            .await;

                        let m = SerializedMessage::new_with_session_and_store(
                            ctx,
                            s_id,
                            s_name,
                            s_cat,
                            s_pfx,
                            s_bn,
                            Some(db_store),
                        );

                        if let Ok(settings) = db_settings.get_settings().await {
                            if settings.auto_typing {
                                let _ = m.send_typing().await;
                            }
                            if settings.auto_recording {
                                let _ = m.send_recording().await;
                            }
                        }

                        if let Err(e) = manager.dispatch(m).await {
                            eprintln!("[dispatch:err] {e}");
                        }
                    }
                }
            })
            .build()
            .await?;

        let client_arc = bot.client();
        *self.client.write().await = Some(client_arc);

        let handle = bot.spawn();
        *handle_guard = Some(handle);

        let _ = self.database.set_session_active(&session_id, true).await;
        println!("[session:{}] Background task spawned", session_id);

        Ok(())
    }

    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let mut handle_guard = self.bot_handle.lock().await;
        if let Some(handle) = handle_guard.take() {
            println!("[session:{}] Stopping session...", self.id().await);
            handle.shutdown().await;
            *self.client.write().await = None;
            *self.status.write().await = SessionStatus::Stopped;
            let _ = self.database.set_session_active(&self.id().await, false).await;
            println!("[session:{}] Stopped successfully", self.id().await);
        } else {
            *self.status.write().await = SessionStatus::Stopped;
        }
        Ok(())
    }

    pub async fn restart(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.stop().await?;
        self.start().await?;
        Ok(())
    }
}
