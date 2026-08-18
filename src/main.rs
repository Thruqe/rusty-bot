mod database;
mod plugins;
mod serialize;
mod session;
mod util;

use database::{Database, SettingsStore};
use plugins::init_plugins;
use session::SessionManager;
use std::sync::Arc;

#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _ = *util::time::START_TIME;

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "whatsapp.db".to_string());
    let db = if db_url.starts_with("postgres://") || db_url.starts_with("postgresql://") {
        Database::new_postgres(&db_url).await?
    } else {
        Database::new_sqlite(&db_url).await?
    };

    let settings = db.get_settings().await?;
    println!(
        "Bot starting: {} (Prefix: '{}', Work Type: '{}')",
        settings.bot_name, settings.prefix, settings.work_type
    );

    let command_manager = Arc::new(init_plugins(&settings.prefix, &settings.bot_name));
    let session_manager = SessionManager::new(db.clone(), command_manager.clone(), "./sessions");

    println!("Initializing multi-session manager...");
    session_manager.init().await?;

    println!("Bot is active. Press Ctrl+C to terminate.");
    tokio::signal::ctrl_c().await?;

    println!("\nShutting down gracefully...");
    session_manager.shutdown_all().await;
    println!("Bot exited cleanly.");

    Ok(())
}
