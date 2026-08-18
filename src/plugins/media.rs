use super::Command;
use crate::util::{
    add_metadata, image_to_webp, is_video_or_gif_bytes, media_to_photo, video_to_animated_webp,
    StickerMetadata,
};

pub fn commands() -> Vec<Command> {
    vec![
        Command::new("sticker", |m, args| async move {
            let (media_bytes, mut is_animated) = match m.download_media().await {
                Ok((bytes, anim)) => (bytes, anim),
                Err(_) => {
                    m.reply("reply to an image, video, or sticker with .sticker").await?;
                    return Ok(());
                }
            };

            if is_video_or_gif_bytes(&media_bytes) {
                is_animated = true;
            }

            // Parse author|packname from args if provided (e.g. .sticker author|packname or .sticker author)
            let (author, pack_name) = if !args.is_empty() {
                let full_arg = args.join(" ");
                if let Some((a, p)) = full_arg.split_once('|') {
                    (
                        if a.trim().is_empty() {
                            if !m.push_name.is_empty() {
                                m.push_name.clone()
                            } else {
                                "wa-bot".to_string()
                            }
                        } else {
                            a.trim().to_string()
                        },
                        if p.trim().is_empty() {
                            m.session_bot_name.clone().unwrap_or_else(|| "wa-bot".to_string())
                        } else {
                            p.trim().to_string()
                        },
                    )
                } else {
                    (
                        full_arg.trim().to_string(),
                        m.session_bot_name.clone().unwrap_or_else(|| "wa-bot".to_string()),
                    )
                }
            } else {
                (
                    if !m.push_name.is_empty() {
                        m.push_name.clone()
                    } else {
                        "wa-bot".to_string()
                    },
                    m.session_bot_name.clone().unwrap_or_else(|| "wa-bot".to_string()),
                )
            };

            let webp_bytes = if is_animated {
                match video_to_animated_webp(&media_bytes).await {
                    Ok(webp) => webp,
                    Err(e) => {
                        eprintln!("[sticker:anim_err] {e}, falling back to static webp conversion");
                        match image_to_webp(&media_bytes) {
                            Ok(webp) => {
                                is_animated = false;
                                webp
                            }
                            Err(_) => media_bytes,
                        }
                    }
                }
            } else {
                match image_to_webp(&media_bytes) {
                    Ok(webp) => webp,
                    Err(_) => media_bytes,
                }
            };

            // Inject WhatsApp Sticker EXIF metadata (pack_name, publisher/author)
            let metadata = StickerMetadata::new(pack_name, author);
            let final_sticker = match add_metadata(&webp_bytes, metadata) {
                Ok(bytes) => bytes,
                Err(_) => webp_bytes,
            };

            m.send_sticker(final_sticker, is_animated).await?;
            Ok(())
        })
        .alias("s")
        .category("media")
        .public(true)
        .hidecommand(false)
        .desc("Convert image, video, or sticker to WhatsApp sticker with EXIF metadata"),

        Command::new("photo", |m, _args| async move {
            let (media_bytes, _) = match m.download_media().await {
                Ok(res) => res,
                Err(_) => {
                    m.reply("reply to a sticker, video, or image with .photo").await?;
                    return Ok(());
                }
            };

            let photo_bytes = match media_to_photo(&media_bytes).await {
                Ok(bytes) => bytes,
                Err(e) => {
                    eprintln!("[photo:err] {e}");
                    m.reply("failed to convert media to photo").await?;
                    return Ok(());
                }
            };

            m.send_image(photo_bytes, None).await?;
            Ok(())
        })
        .alias("toimg")
        .category("media")
        .public(true)
        .hidecommand(false)
        .desc("Convert sticker or video to photo image"),
    ]
}
