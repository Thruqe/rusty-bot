use image::ImageEncoder;
use img_parts::webp::WebP;
use img_parts::{Bytes, ImageEXIF};
use serde::{Deserialize, Serialize};

/// EXIF header for WhatsApp sticker metadata.
const EXIF_HEADER: [u8; 22] = [
    0x49, 0x49, 0x2A, 0x00, // Little-endian TIFF
    0x08, 0x00, 0x00, 0x00, // Offset to IFD
    0x01, 0x00, // Number of entries
    0x41, 0x57, // Tag ID (WhatsApp custom)
    0x07, 0x00, // Type (UNDEFINED)
    0x00, 0x00, 0x00, 0x00, // Count (to be filled)
    0x16, 0x00, 0x00, 0x00, // Offset to data
];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StickerMetadata {
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub pack_id: String,
    pub pack_name: String,
    pub publisher: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub emojis: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub android_app_store_link: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ios_app_store_link: Option<String>,
}

#[derive(Serialize)]
struct ExifStickerMetadataRef<'a> {
    #[serde(rename = "sticker-pack-id")]
    pack_id: &'a str,
    #[serde(rename = "sticker-pack-name")]
    pack_name: &'a str,
    #[serde(rename = "sticker-pack-publisher")]
    publisher: &'a str,
    #[serde(skip_serializing_if = "<[String]>::is_empty")]
    emojis: &'a [String],
    #[serde(
        skip_serializing_if = "Option::is_none",
        rename = "android-app-store-link"
    )]
    android_app_store_link: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "ios-app-store-link")]
    ios_app_store_link: Option<&'a str>,
}

#[derive(Deserialize)]
struct ExifStickerMetadataOwned {
    #[serde(default, rename = "sticker-pack-id")]
    pack_id: Option<String>,
    #[serde(default, rename = "sticker-pack-name")]
    pack_name: Option<String>,
    #[serde(default, rename = "sticker-pack-publisher")]
    publisher: Option<String>,
    #[serde(default)]
    emojis: Vec<String>,
    #[serde(default, rename = "android-app-store-link")]
    android_app_store_link: Option<String>,
    #[serde(default, rename = "ios-app-store-link")]
    ios_app_store_link: Option<String>,
}

impl From<ExifStickerMetadataOwned> for StickerMetadata {
    fn from(m: ExifStickerMetadataOwned) -> Self {
        Self {
            pack_id: m.pack_id.unwrap_or_default(),
            pack_name: m.pack_name.unwrap_or_default(),
            publisher: m.publisher.unwrap_or_default(),
            emojis: m.emojis,
            android_app_store_link: m.android_app_store_link,
            ios_app_store_link: m.ios_app_store_link,
        }
    }
}

impl StickerMetadata {
    pub fn new(pack_name: impl Into<String>, publisher: impl Into<String>) -> Self {
        Self {
            pack_id: uuid::Uuid::new_v4().to_string(),
            pack_name: pack_name.into(),
            publisher: publisher.into(),
            emojis: Vec::new(),
            android_app_store_link: None,
            ios_app_store_link: None,
        }
    }

    #[inline]
    fn ensure_pack_id(&mut self) {
        if self.pack_id.is_empty() {
            self.pack_id = uuid::Uuid::new_v4().to_string();
        }
    }

    #[inline]
    fn build_exif(&self) -> Result<Vec<u8>, serde_json::Error> {
        let exif_meta = ExifStickerMetadataRef {
            pack_id: &self.pack_id,
            pack_name: &self.pack_name,
            publisher: &self.publisher,
            emojis: &self.emojis,
            android_app_store_link: self.android_app_store_link.as_deref(),
            ios_app_store_link: self.ios_app_store_link.as_deref(),
        };
        let json = serde_json::to_vec(&exif_meta)?;
        let json_len = json.len() as u32;

        let mut exif = Vec::with_capacity(EXIF_HEADER.len() + json.len());
        exif.extend_from_slice(&EXIF_HEADER);
        exif.extend_from_slice(&json);

        // Write JSON length at offset 14 (little-endian u32)
        exif[14..18].copy_from_slice(&json_len.to_le_bytes());

        Ok(exif)
    }
}

#[allow(dead_code)]
#[inline]
fn is_whatsapp_sticker_exif(exif_bytes: &[u8]) -> bool {
    matches!(
        (exif_bytes.get(0..4), exif_bytes.get(10..12)),
        (Some(&[0x49, 0x49, 0x2A, 0x00]), Some(&[0x41, 0x57]))
    )
}

/// Add sticker metadata to a WebP image, returning the modified bytes.
pub fn add_metadata(webp_data: &[u8], mut metadata: StickerMetadata) -> Result<Vec<u8>, String> {
    metadata.ensure_pack_id();

    let mut webp = WebP::from_bytes(Bytes::copy_from_slice(webp_data))
        .map_err(|e| format!("Invalid WebP: {e}"))?;

    let exif_data = metadata
        .build_exif()
        .map_err(|e| format!("Failed to serialize metadata: {e}"))?;
    webp.set_exif(Some(Bytes::from(exif_data)));

    Ok(webp.encoder().bytes().to_vec())
}

/// Extract sticker metadata from a WebP image, if present.
#[allow(dead_code)]
pub fn get_metadata(webp_data: &[u8]) -> Result<Option<StickerMetadata>, String> {
    let webp = WebP::from_bytes(Bytes::copy_from_slice(webp_data))
        .map_err(|e| format!("Invalid WebP: {e}"))?;

    let Some(exif_bytes) = webp.exif() else {
        return Ok(None);
    };

    if !is_whatsapp_sticker_exif(&exif_bytes) {
        return Ok(None);
    }

    if exif_bytes.len() <= EXIF_HEADER.len() {
        return Ok(None);
    }

    let json_bytes = &exif_bytes[EXIF_HEADER.len()..];

    let Ok(exif_meta) = serde_json::from_slice::<ExifStickerMetadataOwned>(json_bytes) else {
        return Ok(None);
    };

    Ok(Some(StickerMetadata::from(exif_meta)))
}

/// Convert any image (JPEG, PNG, WebP) to 512x512 square WebP image bytes.
pub fn image_to_webp(image_bytes: &[u8]) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let img = image::load_from_memory(image_bytes)?;
    let resized = img.resize_to_fill(512, 512, image::imageops::FilterType::Lanczos3);
    let rgba = resized.to_rgba8();

    let mut output = Vec::new();
    let encoder = image::codecs::webp::WebPEncoder::new_lossless(&mut output);
    encoder.write_image(
        &rgba,
        rgba.width(),
        rgba.height(),
        image::ExtendedColorType::Rgba8,
    )?;
    Ok(output)
}

/// Check if raw buffer bytes match video/GIF container signatures
pub fn is_video_or_gif_bytes(bytes: &[u8]) -> bool {
    if bytes.len() < 12 {
        return false;
    }
    // GIF
    if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        return true;
    }
    // MP4 / MOV (ftyp box)
    if bytes.len() >= 8 && (bytes[4..8] == *b"ftyp" || bytes[4..8] == *b"moov") {
        return true;
    }
    // WebM / MKV (EBML ID: 0x1A45DFA3)
    if bytes.starts_with(&[0x1A, 0x45, 0xDF, 0xA3]) {
        return true;
    }
    // Animated WebP (RIFF....WEBP with ANIM chunk)
    if bytes.starts_with(b"RIFF") && bytes.len() >= 12 && &bytes[8..12] == b"WEBP" {
        if bytes.windows(4).any(|w| w == b"ANIM") {
            return true;
        }
    }
    false
}

/// Convert video/GIF bytes to 512x512 animated WebP using ffmpeg
pub async fn video_to_animated_webp(
    video_bytes: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    let id = uuid::Uuid::new_v4().to_string();
    let in_path = std::env::temp_dir().join(format!("wa_vid_in_{id}.tmp"));
    let out_path = std::env::temp_dir().join(format!("wa_vid_out_{id}.webp"));

    tokio::fs::write(&in_path, video_bytes).await?;

    let status = tokio::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(&in_path)
        .arg("-t")
        .arg("6")
        .arg("-vf")
        .arg("scale=512:512:force_original_aspect_ratio=decrease,fps=15,pad=512:512:(512-iw)/2:(512-ih)/2:color=0x00000000")
        .arg("-c:v")
        .arg("libwebp")
        .arg("-loop")
        .arg("0")
        .arg("-an")
        .arg("-s")
        .arg("512:512")
        .arg("-quality")
        .arg("60")
        .arg(&out_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await?;

    let _ = tokio::fs::remove_file(&in_path).await;

    if !status.success() {
        let _ = tokio::fs::remove_file(&out_path).await;
        return Err("ffmpeg failed to convert video to animated webp".into());
    }

    let webp_data = tokio::fs::read(&out_path).await?;
    let _ = tokio::fs::remove_file(&out_path).await;

    Ok(webp_data)
}

/// Convert sticker / video / GIF media to JPEG photo bytes
pub async fn media_to_photo(
    media_bytes: &[u8],
) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
    // 1. Try decoding with image crate directly (works for static WebP / PNG / JPEG)
    if let Ok(img) = image::load_from_memory(media_bytes) {
        let rgb = img.to_rgb8();
        let mut output = Vec::new();
        let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut output, 90);
        if encoder.encode_image(&rgb).is_ok() {
            return Ok(output);
        }
    }

    // 2. Fallback to ffmpeg for animated WebP or video containers
    let id = uuid::Uuid::new_v4().to_string();
    let in_path = std::env::temp_dir().join(format!("wa_photo_in_{id}.tmp"));
    let out_path = std::env::temp_dir().join(format!("wa_photo_out_{id}.jpg"));

    tokio::fs::write(&in_path, media_bytes).await?;

    let status = tokio::process::Command::new("ffmpeg")
        .arg("-y")
        .arg("-i")
        .arg(&in_path)
        .arg("-vframes")
        .arg("1")
        .arg("-q:v")
        .arg("2")
        .arg(&out_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .await?;

    let _ = tokio::fs::remove_file(&in_path).await;

    if !status.success() {
        let _ = tokio::fs::remove_file(&out_path).await;
        return Err("ffmpeg failed to convert media to photo".into());
    }

    let photo_data = tokio::fs::read(&out_path).await?;
    let _ = tokio::fs::remove_file(&out_path).await;

    Ok(photo_data)
}
