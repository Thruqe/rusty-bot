use std::sync::Arc;
use whatsapp_rust::Jid;
use whatsapp_rust::prelude::wa::{Message, MessageKey};
use whatsapp_rust::prelude::*;
use whatsapp_rust::wacore_binary::JidExt;

pub type MessageId = String;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    Image,
    Video,
    Audio,
    Voice,
    Document,
    Sticker,
    Contact,
    Location,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct SerializedQuoted {
    pub id: String,
    pub sender: Jid,
    pub sender_number: String,
    pub text: Option<String>,
    pub is_from_me: bool,
    pub is_group: bool,
    pub message: Option<Box<Message>>,
    pub media_type: Option<MediaType>,
    pub mentioned_jid: Vec<String>,
}

#[allow(dead_code)]
impl SerializedQuoted {
    #[inline(always)]
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    #[inline(always)]
    pub fn text_or_empty(&self) -> &str {
        self.text.as_deref().unwrap_or("")
    }

    #[inline(always)]
    pub fn has_text(&self) -> bool {
        self.text.as_ref().map(|s| !s.is_empty()).unwrap_or(false)
    }

    #[inline(always)]
    pub fn sender_number(&self) -> &str {
        &self.sender_number
    }

    #[inline(always)]
    pub fn is_image(&self) -> bool {
        matches!(self.media_type, Some(MediaType::Image))
    }

    #[inline(always)]
    pub fn is_video(&self) -> bool {
        matches!(self.media_type, Some(MediaType::Video))
    }

    #[inline(always)]
    pub fn is_audio(&self) -> bool {
        matches!(self.media_type, Some(MediaType::Audio | MediaType::Voice))
    }

    #[inline(always)]
    pub fn is_voice(&self) -> bool {
        matches!(self.media_type, Some(MediaType::Voice))
    }

    #[inline(always)]
    pub fn is_document(&self) -> bool {
        matches!(self.media_type, Some(MediaType::Document))
    }

    #[inline(always)]
    pub fn is_sticker(&self) -> bool {
        matches!(self.media_type, Some(MediaType::Sticker))
    }

    #[inline(always)]
    pub fn is_contact(&self) -> bool {
        matches!(self.media_type, Some(MediaType::Contact))
    }

    #[inline(always)]
    pub fn is_location(&self) -> bool {
        matches!(self.media_type, Some(MediaType::Location))
    }

    pub fn inner_quoted_message(&self) -> Option<Message> {
        let msg = self.message.as_ref()?;
        let base = SerializedMessage::get_base_message(msg);
        let ci = SerializedMessage::extract_context_info(base);
        println!(
            "[debug:inner_quote] checking replied msg (id={}): has_context_info={}",
            self.id,
            ci.is_some()
        );
        if let Some(ci) = ci {
            let has_quoted_msg = ci.quoted_message.is_set();
            println!(
                "[debug:inner_quote] context_info stanza_id={:?}, participant={:?}, quoted_message_set={}",
                ci.stanza_id, ci.participant, has_quoted_msg
            );
            let qm = ci.quoted_message.as_option().cloned();
            if let Some(ref m) = qm {
                println!(
                    "[debug:inner_quote] found inner quoted message (conv={:?}, ext={:?})",
                    m.conversation, m.extended_text_message.text
                );
            }
            qm
        } else {
            None
        }
    }

    pub fn inner_quoted(&self, chat: &Jid) -> Option<SerializedQuoted> {
        let msg = self.message.as_ref()?;
        let base = SerializedMessage::get_base_message(msg);
        SerializedMessage::extract_quoted(base, chat)
    }

    pub fn key(&self, chat: &Jid) -> MessageKey {
        let needs_participant = chat.is_group() || chat.is_status_broadcast();
        MessageKey {
            remote_jid: Some(chat.to_string()),
            from_me: Some(self.is_from_me),
            id: Some(self.id.clone()),
            participant: if needs_participant {
                Some(self.sender.to_string())
            } else {
                None
            },
        }
    }
}

use crate::database::{Database, MessageStore};

#[allow(dead_code)]
pub struct SerializedMessage {
    pub ctx: MessageContext,
    pub id: MessageId,
    pub sender: Jid,
    pub chat: Jid,
    pub push_name: String,
    pub text: Option<String>,
    pub is_from_me: bool,
    pub is_group: bool,
    pub timestamp_ms: i64,
    pub quoted: Option<SerializedQuoted>,
    pub mentions: Vec<Jid>,
    pub mentioned_jid: Vec<String>,
    pub media_type: Option<MediaType>,
    // Session metadata
    pub session_id: String,
    pub session_name: String,
    pub session_category: String,
    pub session_prefix: Option<String>,
    pub session_bot_name: Option<String>,
    // Persistent database store
    pub db: Option<Arc<Database>>,
}

#[allow(dead_code)]
impl SerializedMessage {
    pub fn get_base_message(msg: &Message) -> &Message {
        let mut current = msg;
        while let Some(wrapper) = current.ephemeral_message.as_option() {
            if let Some(inner) = wrapper.message.as_option() {
                current = inner;
            } else {
                break;
            }
        }
        while let Some(wrapper) = current.view_once_message.as_option() {
            if let Some(inner) = wrapper.message.as_option() {
                current = inner;
            } else {
                break;
            }
        }
        while let Some(wrapper) = current.view_once_message_v2.as_option() {
            if let Some(inner) = wrapper.message.as_option() {
                current = inner;
            } else {
                break;
            }
        }
        while let Some(wrapper) = current.document_with_caption_message.as_option() {
            if let Some(inner) = wrapper.message.as_option() {
                current = inner;
            } else {
                break;
            }
        }
        if let Some(dsm) = current.device_sent_message.as_option() {
            if let Some(inner) = dsm.message.as_option() {
                current = inner;
            }
        }
        current
    }

    pub fn new(ctx: MessageContext) -> Self {
        Self::new_with_session(
            ctx,
            "default".to_string(),
            "Default Session".to_string(),
            "default".to_string(),
            None,
            None,
        )
    }

    pub fn new_with_session(
        ctx: MessageContext,
        session_id: String,
        session_name: String,
        session_category: String,
        session_prefix: Option<String>,
        session_bot_name: Option<String>,
    ) -> Self {
        Self::new_with_session_and_store(
            ctx,
            session_id,
            session_name,
            session_category,
            session_prefix,
            session_bot_name,
            None,
        )
    }

    pub fn new_with_session_and_store(
        ctx: MessageContext,
        session_id: String,
        session_name: String,
        session_category: String,
        session_prefix: Option<String>,
        session_bot_name: Option<String>,
        db: Option<Arc<Database>>,
    ) -> Self {
        let base_msg = Self::get_base_message(&ctx.message);
        let text = Self::extract_text(base_msg);
        let id = ctx.info.id.to_string();

        let chat = ctx.info.source.chat.clone();
        let sender = ctx.info.source.sender.clone();
        let is_from_me = ctx.info.source.is_from_me;
        let is_group = chat.is_group();

        let push_name = ctx.info.push_name.clone();
        let timestamp_ms = ctx.info.timestamp.timestamp_millis();
        let media_type = Self::extract_media_type(base_msg);
        let quoted = Self::extract_quoted(base_msg, &chat);

        println!(
            "[debug:msg_in] id={} from={} chat={} text={:?} has_quoted={}",
            id,
            sender,
            chat,
            text,
            quoted.is_some()
        );
        if let Some(ref q) = quoted {
            println!(
                "[debug:quoted_in] quoted_id={} quoted_sender={} quoted_text={:?}",
                q.id,
                q.sender,
                q.text
            );
        }

        let (mentions, mentioned_jid) = if let Some(ci) = Self::extract_context_info(base_msg) {
            let jids = ci
                .mentioned_jid
                .iter()
                .filter_map(|s| s.parse::<Jid>().ok())
                .collect();
            (jids, ci.mentioned_jid.clone())
        } else {
            (Vec::new(), Vec::new())
        };

        Self {
            ctx,
            id,
            sender,
            chat,
            push_name,
            text,
            is_from_me,
            is_group,
            timestamp_ms,
            quoted,
            mentions,
            mentioned_jid,
            media_type,
            session_id,
            session_name,
            session_category,
            session_prefix,
            session_bot_name,
            db,
        }
    }

    pub fn extract_context_info(msg: &Message) -> Option<&whatsapp_rust::prelude::wa::ContextInfo> {
        let msg = Self::get_base_message(msg);
        if let Some(ref ext) = msg.extended_text_message.as_option() {
            if let Some(ref ci) = ext.context_info.as_option() {
                return Some(ci);
            }
        }
        if let Some(ref img) = msg.image_message.as_option() {
            if let Some(ref ci) = img.context_info.as_option() {
                return Some(ci);
            }
        }
        if let Some(ref vid) = msg.video_message.as_option() {
            if let Some(ref ci) = vid.context_info.as_option() {
                return Some(ci);
            }
        }
        if let Some(ref doc) = msg.document_message.as_option() {
            if let Some(ref ci) = doc.context_info.as_option() {
                return Some(ci);
            }
        }
        if let Some(ref aud) = msg.audio_message.as_option() {
            if let Some(ref ci) = aud.context_info.as_option() {
                return Some(ci);
            }
        }
        if let Some(ref stk) = msg.sticker_message.as_option() {
            if let Some(ref ci) = stk.context_info.as_option() {
                return Some(ci);
            }
        }
        if let Some(ref loc) = msg.location_message.as_option() {
            if let Some(ref ci) = loc.context_info.as_option() {
                return Some(ci);
            }
        }
        if let Some(ref con) = msg.contact_message.as_option() {
            if let Some(ref ci) = con.context_info.as_option() {
                return Some(ci);
            }
        }
        if let Some(ref btn) = msg.buttons_response_message.as_option() {
            if let Some(ref ci) = btn.context_info.as_option() {
                return Some(ci);
            }
        }
        if let Some(ref list) = msg.list_response_message.as_option() {
            if let Some(ref ci) = list.context_info.as_option() {
                return Some(ci);
            }
        }
        if let Some(ref tpl) = msg.template_button_reply_message.as_option() {
            if let Some(ref ci) = tpl.context_info.as_option() {
                return Some(ci);
            }
        }
        if let Some(ref doc_cap) = msg.document_with_caption_message.as_option() {
            if let Some(ref doc_msg) = doc_cap.message.as_option() {
                if let Some(ref doc) = doc_msg.document_message.as_option() {
                    if let Some(ref ci) = doc.context_info.as_option() {
                        return Some(ci);
                    }
                }
            }
        }
        None
    }

    pub fn extract_media_type(msg: &Message) -> Option<MediaType> {
        let msg = Self::get_base_message(msg);
        if msg.image_message.is_set() {
            return Some(MediaType::Image);
        }
        if msg.video_message.is_set() {
            return Some(MediaType::Video);
        }
        if msg.audio_message.is_set() {
            if msg.audio_message.ptt.unwrap_or(false) {
                return Some(MediaType::Voice);
            }
            return Some(MediaType::Audio);
        }
        if msg.document_message.is_set() || msg.document_with_caption_message.is_set() {
            return Some(MediaType::Document);
        }
        if msg.sticker_message.is_set() {
            return Some(MediaType::Sticker);
        }
        if msg.contact_message.is_set() || msg.contacts_array_message.is_set() {
            return Some(MediaType::Contact);
        }
        if msg.location_message.is_set() || msg.live_location_message.is_set() {
            return Some(MediaType::Location);
        }
        None
    }

    pub fn extract_quoted(msg: &Message, chat: &Jid) -> Option<SerializedQuoted> {
        let base = Self::get_base_message(msg);
        let ci = Self::extract_context_info(base)?;
        let quoted_msg = ci.quoted_message.as_option()?;
        let stanza_id = ci.stanza_id.clone().unwrap_or_default();

        let sender = if let Some(ref p) = ci.participant {
            p.parse::<Jid>().unwrap_or_else(|_| chat.clone())
        } else {
            chat.clone()
        };

        let sender_number = sender.user().to_string();
        let text = Self::extract_text(quoted_msg);
        let media_type = Self::extract_media_type(quoted_msg);
        let mentioned_jid = ci.mentioned_jid.clone();

        Some(SerializedQuoted {
            id: stanza_id,
            sender,
            sender_number,
            text,
            is_from_me: ci.is_forwarded.unwrap_or(false),
            is_group: chat.is_group(),
            message: Some(Box::new(quoted_msg.clone())),
            media_type,
            mentioned_jid,
        })
    }

    pub fn extract_text(msg: &Message) -> Option<String> {
        let msg = Self::get_base_message(msg);
        if let Some(ref s) = msg.conversation {
            if !s.is_empty() {
                return Some(s.clone());
            }
        }
        if let Some(ref s) = msg.extended_text_message.text {
            if !s.is_empty() {
                return Some(s.clone());
            }
        }
        if let Some(ref s) = msg.image_message.caption {
            if !s.is_empty() {
                return Some(s.clone());
            }
        }
        if let Some(ref s) = msg.video_message.caption {
            if !s.is_empty() {
                return Some(s.clone());
            }
        }
        if let Some(ref s) = msg.document_message.caption {
            if !s.is_empty() {
                return Some(s.clone());
            }
        }
        if let Some(ref s) = msg
            .document_with_caption_message
            .message
            .document_message
            .caption
        {
            if !s.is_empty() {
                return Some(s.clone());
            }
        }
        if let Some(ref s) = msg.buttons_response_message.selected_button_id {
            if !s.is_empty() {
                return Some(s.clone());
            }
        }
        if let Some(ref s) = msg.list_response_message.title {
            if !s.is_empty() {
                return Some(s.clone());
            }
        }
        if let Some(ref s) = msg.template_button_reply_message.selected_id {
            if !s.is_empty() {
                return Some(s.clone());
            }
        }
        None
    }

    #[inline(always)]
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }

    #[inline(always)]
    pub fn text_or_empty(&self) -> &str {
        self.text.as_deref().unwrap_or("")
    }

    #[inline(always)]
    pub fn quoted(&self) -> Option<&SerializedQuoted> {
        self.quoted.as_ref()
    }

    #[inline(always)]
    pub fn has_quoted(&self) -> bool {
        self.quoted.is_some()
    }

    #[inline(always)]
    pub fn quoted_text(&self) -> Option<&str> {
        self.quoted.as_ref().and_then(|q| q.text())
    }

    pub async fn quoted_of_replied(&self) -> Option<Message> {
        let q = self.quoted.as_ref()?;
        println!("[debug:quoted_lookup] looking up replied message id={}", q.id);

        let mut replied_msg: Option<Message> = None;

        if let Some(ref db) = self.db {
            if let Some(msg) = db.get_message(&q.id).await {
                println!("[debug:quoted_lookup] found replied message in database store");
                replied_msg = Some(msg);
            }
        }

        if replied_msg.is_none() {
            println!(
                "[debug:quoted_lookup] replied message id={} not in database store, checking attached message payload",
                q.id
            );
            replied_msg = q.message.as_deref().cloned();
        }

        if let Some(msg) = replied_msg {
            let base = Self::get_base_message(&msg);
            if let Some(ci) = Self::extract_context_info(base) {
                println!(
                    "[debug:quoted_lookup] context_info found in replied msg: stanza_id={:?}, participant={:?}, quoted_message_set={}",
                    ci.stanza_id, ci.participant, ci.quoted_message.is_set()
                );
                if let Some(qm) = ci.quoted_message.as_option() {
                    println!("[debug:quoted_lookup] found inner quoted message from replied message!");
                    return Some(qm.clone());
                } else {
                    println!("[debug:quoted_lookup] replied message context_info does not have quoted_message");
                }
            } else {
                println!("[debug:quoted_lookup] replied message does not have context_info");
            }
        }
        None
    }

    #[inline(always)]
    pub fn sender_number(&self) -> &str {
        self.sender.user()
    }

    #[inline(always)]
    pub fn chat_number(&self) -> &str {
        self.chat.user()
    }

    #[inline(always)]
    pub fn sender_name(&self) -> &str {
        if !self.push_name.is_empty() {
            &self.push_name
        } else {
            self.sender.user()
        }
    }

    #[inline(always)]
    pub fn is_group(&self) -> bool {
        self.is_group
    }

    #[inline(always)]
    pub fn is_private(&self) -> bool {
        !self.is_group
    }

    #[inline(always)]
    pub fn is_from_me(&self) -> bool {
        self.is_from_me
    }

    #[inline(always)]
    pub fn is_owner(&self) -> bool {
        self.is_from_me
    }

    #[inline(always)]
    pub fn is_media(&self) -> bool {
        self.media_type.is_some()
    }

    #[inline(always)]
    pub fn is_image(&self) -> bool {
        matches!(self.media_type, Some(MediaType::Image))
    }

    #[inline(always)]
    pub fn is_video(&self) -> bool {
        matches!(self.media_type, Some(MediaType::Video))
    }

    #[inline(always)]
    pub fn is_audio(&self) -> bool {
        matches!(self.media_type, Some(MediaType::Audio | MediaType::Voice))
    }

    #[inline(always)]
    pub fn is_voice(&self) -> bool {
        matches!(self.media_type, Some(MediaType::Voice))
    }

    #[inline(always)]
    pub fn is_document(&self) -> bool {
        matches!(self.media_type, Some(MediaType::Document))
    }

    #[inline(always)]
    pub fn is_sticker(&self) -> bool {
        matches!(self.media_type, Some(MediaType::Sticker))
    }

    #[inline(always)]
    pub fn is_contact(&self) -> bool {
        matches!(self.media_type, Some(MediaType::Contact))
    }

    #[inline(always)]
    pub fn is_location(&self) -> bool {
        matches!(self.media_type, Some(MediaType::Location))
    }

    #[inline(always)]
    pub fn is_mentioned(&self, jid: &Jid) -> bool {
        self.mentions.contains(jid)
    }

    #[inline(always)]
    pub fn media_type(&self) -> Option<MediaType> {
        self.media_type
    }

    pub fn key(&self) -> MessageKey {
        MessageKey {
            remote_jid: Some(self.chat.to_string()),
            from_me: Some(self.is_from_me),
            id: Some(self.id.clone()),
            participant: if self.is_group {
                Some(self.sender.to_string())
            } else {
                None
            },
        }
    }

    pub fn quoted_key(&self) -> Option<MessageKey> {
        self.quoted.as_ref().map(|q| q.key(&self.chat))
    }

    /// Reply to current chat with plain text
    pub async fn reply<T: AsRef<str>>(
        &self,
        content: T,
    ) -> Result<SendResult, Box<dyn std::error::Error + Send + Sync>> {
        let res = self.ctx.reply(content.as_ref()).await?;
        Ok(res)
    }

    /// Reply quoting the current message directly
    pub async fn reply_quote<T: AsRef<str>>(
        &self,
        content: T,
    ) -> Result<SendResult, Box<dyn std::error::Error + Send + Sync>> {
        let res = self.ctx.reply_quoting(content.as_ref()).await?;
        Ok(res)
    }

    /// Reply quoting the quoted message if present, or fallback to quoting current message
    pub async fn reply_to_quoted<T: AsRef<str>>(
        &self,
        content: T,
    ) -> Result<SendResult, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref q) = self.quoted {
            let quoted_msg = q.message.as_deref().cloned().unwrap_or_default();
            let context = whatsapp_rust::wacore::proto_helpers::build_quote_context_with_info(
                &q.id,
                &q.sender,
                &self.chat,
                &self.chat,
                &quoted_msg,
            );
            let msg = Message::text_with_context(content.as_ref(), context);
            let res = self.ctx.send_message(msg).await?;
            Ok(res)
        } else {
            self.reply_quote(content).await
        }
    }

    /// Send raw or prepared Message
    pub async fn send_message(
        &self,
        message: Message,
    ) -> Result<SendResult, Box<dyn std::error::Error + Send + Sync>> {
        let res = self.ctx.send_message(message).await?;
        Ok(res)
    }

    /// Edit a previously sent message
    pub async fn edit<T: AsRef<str>>(
        &self,
        target_id: &str,
        new_content: T,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        let new_message = Message {
            conversation: Some(new_content.as_ref().to_string()),
            ..Default::default()
        };

        let res = self
            .ctx
            .client
            .edit_message(&self.chat, target_id, new_message)
            .await?;
        Ok(res)
    }

    /// React with emoji to this message
    pub async fn react<T: AsRef<str>>(
        &self,
        emoji: T,
    ) -> Result<SendResult, Box<dyn std::error::Error + Send + Sync>> {
        let key = self.key();
        let res = self
            .ctx
            .client
            .send_reaction(&self.chat, key, emoji.as_ref())
            .await?;
        Ok(res)
    }

    /// React with emoji to the quoted message if one exists
    pub async fn react_quoted<T: AsRef<str>>(
        &self,
        emoji: T,
    ) -> Result<Option<SendResult>, Box<dyn std::error::Error + Send + Sync>> {
        if let Some(ref q) = self.quoted {
            let key = q.key(&self.chat);
            let res = self
                .ctx
                .client
                .send_reaction(&self.chat, key, emoji.as_ref())
                .await?;
            Ok(Some(res))
        } else {
            Ok(None)
        }
    }

    /// Send typing (composing) chat state indicator
    pub async fn send_typing(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.ctx
            .client
            .chatstate()
            .send_composing(&self.chat)
            .await?;
        Ok(())
    }

    /// Send recording chat state indicator
    pub async fn send_recording(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.ctx
            .client
            .chatstate()
            .send_recording(&self.chat)
            .await?;
        Ok(())
    }

    /// Send paused chat state indicator
    pub async fn send_paused(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.ctx
            .client
            .chatstate()
            .send_paused(&self.chat)
            .await?;
        Ok(())
    }

    /// Download media attached to the current message or quoted message, returning (bytes, is_animated_or_video)
    pub async fn download_media(&self) -> Result<(Vec<u8>, bool), Box<dyn std::error::Error + Send + Sync>> {
        let base = Self::get_base_message(&self.ctx.message);
        if let Some(img) = base.image_message.as_option() {
            let data = self.ctx.client.download(img).await?;
            return Ok((data, false));
        }
        if let Some(sticker) = base.sticker_message.as_option() {
            let data = self.ctx.client.download(sticker).await?;
            return Ok((data, sticker.is_animated.unwrap_or(false)));
        }
        if let Some(doc) = base.document_message.as_option() {
            let data = self.ctx.client.download(doc).await?;
            let is_anim = doc
                .mimetype
                .as_deref()
                .map(|m| m.starts_with("video/") || m == "image/gif")
                .unwrap_or(false);
            return Ok((data, is_anim));
        }
        if let Some(vid) = base.video_message.as_option() {
            let data = self.ctx.client.download(vid).await?;
            return Ok((data, true));
        }

        if let Some(ref q) = self.quoted {
            if let Some(ref qmsg) = q.message {
                let qbase = Self::get_base_message(qmsg);
                if let Some(img) = qbase.image_message.as_option() {
                    let data = self.ctx.client.download(img).await?;
                    return Ok((data, false));
                }
                if let Some(sticker) = qbase.sticker_message.as_option() {
                    let data = self.ctx.client.download(sticker).await?;
                    return Ok((data, sticker.is_animated.unwrap_or(false)));
                }
                if let Some(doc) = qbase.document_message.as_option() {
                    let data = self.ctx.client.download(doc).await?;
                    let is_anim = doc
                        .mimetype
                        .as_deref()
                        .map(|m| m.starts_with("video/") || m == "image/gif")
                        .unwrap_or(false);
                    return Ok((data, is_anim));
                }
                if let Some(vid) = qbase.video_message.as_option() {
                    let data = self.ctx.client.download(vid).await?;
                    return Ok((data, true));
                }
            }

            if let Some(ref db) = self.db {
                if let Some(stored) = db.get_message(&q.id).await {
                    let qbase = Self::get_base_message(&stored);
                    if let Some(img) = qbase.image_message.as_option() {
                        let data = self.ctx.client.download(img).await?;
                        return Ok((data, false));
                    }
                    if let Some(sticker) = qbase.sticker_message.as_option() {
                        let data = self.ctx.client.download(sticker).await?;
                        return Ok((data, sticker.is_animated.unwrap_or(false)));
                    }
                    if let Some(doc) = qbase.document_message.as_option() {
                        let data = self.ctx.client.download(doc).await?;
                        let is_anim = doc
                            .mimetype
                            .as_deref()
                            .map(|m| m.starts_with("video/") || m == "image/gif")
                            .unwrap_or(false);
                        return Ok((data, is_anim));
                    }
                    if let Some(vid) = qbase.video_message.as_option() {
                        let data = self.ctx.client.download(vid).await?;
                        return Ok((data, true));
                    }
                }
            }
        }

        Err("No media found in message or quoted message".into())
    }

    /// Upload and send a WebP sticker
    pub async fn send_sticker(
        &self,
        webp_data: Vec<u8>,
        is_animated: bool,
    ) -> Result<SendResult, Box<dyn std::error::Error + Send + Sync>> {
        use whatsapp_rust::upload::UploadOptions;
        use whatsapp_rust::wacore::download::MediaType as WaMediaType;

        let upload_resp = self
            .ctx
            .client
            .upload(webp_data, WaMediaType::Sticker, UploadOptions::default())
            .await?;

        let sticker_msg = whatsapp_rust::prelude::wa::message::StickerMessage {
            url: Some(upload_resp.url),
            direct_path: Some(upload_resp.direct_path),
            media_key: Some(upload_resp.media_key.to_vec()),
            file_sha256: Some(upload_resp.file_sha256.to_vec()),
            file_enc_sha256: Some(upload_resp.file_enc_sha256.to_vec()),
            file_length: Some(upload_resp.file_length),
            media_key_timestamp: Some(upload_resp.media_key_timestamp),
            mimetype: Some("image/webp".to_string()),
            is_animated: Some(is_animated),
            height: Some(512),
            width: Some(512),
            ..Default::default()
        };

        let mut msg = Message::default();
        msg.sticker_message = Some(sticker_msg).into();
        self.send_message(msg).await
    }

    /// Upload and send a JPEG/PNG image
    pub async fn send_image(
        &self,
        image_bytes: Vec<u8>,
        caption: Option<&str>,
    ) -> Result<SendResult, Box<dyn std::error::Error + Send + Sync>> {
        use whatsapp_rust::upload::UploadOptions;
        use whatsapp_rust::wacore::download::MediaType as WaMediaType;

        let upload_resp = self
            .ctx
            .client
            .upload(image_bytes, WaMediaType::Image, UploadOptions::default())
            .await?;

        let img_msg = whatsapp_rust::prelude::wa::message::ImageMessage {
            url: Some(upload_resp.url),
            direct_path: Some(upload_resp.direct_path),
            media_key: Some(upload_resp.media_key.to_vec()),
            file_sha256: Some(upload_resp.file_sha256.to_vec()),
            file_enc_sha256: Some(upload_resp.file_enc_sha256.to_vec()),
            file_length: Some(upload_resp.file_length),
            media_key_timestamp: Some(upload_resp.media_key_timestamp),
            mimetype: Some("image/jpeg".to_string()),
            caption: caption.map(|s| s.to_string()),
            ..Default::default()
        };

        let mut msg = Message::default();
        msg.image_message = Some(img_msg).into();
        self.send_message(msg).await
    }
}
