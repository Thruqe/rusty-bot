use wa_bot::database::{Database, SessionModel, SessionStore, SettingsStore};
use wa_bot::plugins::init_plugins;
use wa_bot::session::SessionManager;
use std::sync::Arc;

#[tokio::test]
async fn test_database_settings_and_sessions() {
    let db = Database::new_sqlite(":memory:").await.expect("create in-memory db");

    // Test default settings
    let settings = db.get_settings().await.expect("get settings");
    assert_eq!(settings.prefix, ".");
    assert_eq!(settings.bot_name, "WaBot");
    assert_eq!(settings.work_type, "public");

    // Update settings
    db.set_prefix("!").await.expect("update prefix");
    db.set_bot_name("SuperBot").await.expect("update bot_name");
    let updated_settings = db.get_settings().await.expect("get updated settings");
    assert_eq!(updated_settings.prefix, "!");
    assert_eq!(updated_settings.bot_name, "SuperBot");

    // Test Sessions CRUD
    let sessions = db.list_sessions().await.expect("list initial sessions");
    assert!(!sessions.is_empty());
    assert_eq!(sessions[0].id, "default");
    assert_eq!(sessions[0].category, "default");

    // Create session in custom category
    let custom_session = SessionModel {
        id: "work_bot".to_string(),
        name: "Work Assistant".to_string(),
        category: "work".to_string(),
        phone_number: Some("1234567890".to_string()),
        prefix: Some("#".to_string()),
        bot_name: Some("WorkBot".to_string()),
        auto_start: false,
        is_active: false,
        created_at: "2026-01-01 00:00:00".to_string(),
        updated_at: "2026-01-01 00:00:00".to_string(),
    };
    db.save_session(&custom_session).await.expect("save session");

    let loaded = db.get_session("work_bot").await.expect("get session").expect("must exist");
    assert_eq!(loaded.name, "Work Assistant");
    assert_eq!(loaded.category, "work");
    assert_eq!(loaded.prefix, Some("#".to_string()));
    assert_eq!(loaded.bot_name, Some("WorkBot".to_string()));

    let work_sessions = db.list_sessions_by_category("work").await.expect("list by category");
    assert_eq!(work_sessions.len(), 1);
    assert_eq!(work_sessions[0].id, "work_bot");

    let categories = db.list_categories().await.expect("list categories");
    assert!(categories.contains(&"default".to_string()));
    assert!(categories.contains(&"work".to_string()));

    // Test SessionManager
    let cmd_mgr = Arc::new(init_plugins(&updated_settings.prefix, &updated_settings.bot_name));
    let session_mgr = SessionManager::new(db.clone(), cmd_mgr, "./target/test_sessions");
    session_mgr.init().await.expect("session mgr init");

    let created = session_mgr
        .create_session("support_bot", "Customer Support", "support", Some("$"), Some("SupportAI"), false)
        .await
        .expect("create session via manager");
    assert_eq!(created.id().await, "support_bot");
    assert_eq!(created.category().await, "support");

    let support_sessions = session_mgr.list_by_category("support").await;
    assert_eq!(support_sessions.len(), 1);
    assert_eq!(support_sessions[0].id, "support_bot");

    // Clean up
    session_mgr.delete_session("support_bot").await.expect("delete session");
    assert!(session_mgr.get_session("support_bot").await.is_none());
}

#[tokio::test]
async fn test_message_store() {
    use wa_bot::database::MessageStore;
    use whatsapp_rust::prelude::wa::Message;

    let db = Database::new_sqlite(":memory:").await.expect("create db");

    let mut msg = Message::default();
    msg.conversation = Some("Hello world".to_string());

    db.save_wa_message("msg_123", "chat_123", "sender_123", &msg)
        .await
        .expect("save wa message to db");

    let loaded = db.get_message("msg_123").await.expect("must load from db");
    assert_eq!(loaded.conversation, Some("Hello world".to_string()));
}

#[test]
fn test_sticker_exif_metadata() {
    use wa_bot::util::{add_metadata, get_metadata, image_to_webp, StickerMetadata};
    use image::{Rgba, RgbaImage};

    // Create a 10x10 blank test image in PNG format
    let img = RgbaImage::from_pixel(10, 10, Rgba([255, 0, 0, 255]));
    let mut png_bytes = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .expect("write test png");

    // Convert to 512x512 WebP
    let webp_bytes = image_to_webp(&png_bytes).expect("convert to webp");

    // Inject WhatsApp sticker EXIF metadata
    let metadata = StickerMetadata::new("WaBot Stickers", "TestAuthor");
    let sticker_with_exif = add_metadata(&webp_bytes, metadata.clone()).expect("add exif metadata");

    // Extract and verify metadata
    let extracted = get_metadata(&sticker_with_exif).expect("get exif").expect("must have metadata");
    assert_eq!(extracted.pack_name, "WaBot Stickers");
    assert_eq!(extracted.publisher, "TestAuthor");
}

#[test]
fn test_animated_sticker_detection() {
    use wa_bot::util::is_video_or_gif_bytes;

    let gif_header = b"GIF89a\x01\x00\x01\x00\x80\x00\x00";
    assert!(is_video_or_gif_bytes(gif_header));

    let mp4_header = b"\x00\x00\x00\x18ftypmp42\x00\x00\x00\x00";
    assert!(is_video_or_gif_bytes(mp4_header));

    let png_header = b"\x89PNG\r\n\x1a\n\x00\x00\x00\rIHDR";
    assert!(!is_video_or_gif_bytes(png_header));
}

#[tokio::test]
async fn test_media_to_photo() {
    use wa_bot::util::{image_to_webp, media_to_photo};
    use image::{Rgba, RgbaImage};

    // Create a 10x10 test image
    let img = RgbaImage::from_pixel(10, 10, Rgba([0, 255, 0, 255]));
    let mut png_bytes = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png_bytes), image::ImageFormat::Png)
        .expect("write test png");

    let webp_bytes = image_to_webp(&png_bytes).expect("convert to webp");
    let photo_bytes = media_to_photo(&webp_bytes).await.expect("convert webp to photo");

    // Verify JPEG header (SOI marker 0xFF, 0xD8)
    assert!(photo_bytes.starts_with(&[0xFF, 0xD8]));
}
