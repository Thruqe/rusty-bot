use async_trait::async_trait;
use whatsapp_rust::buffa::Message as ProtoMessage;
use whatsapp_rust::prelude::wa::Message;

#[allow(dead_code)]
#[async_trait]
pub trait MessageStore: Send + Sync {
    async fn save_message_bytes(
        &self,
        id: &str,
        chat: &str,
        sender: &str,
        bytes: &[u8],
    ) -> Result<(), sqlx::Error>;

    async fn get_message_bytes(&self, id: &str) -> Result<Option<Vec<u8>>, sqlx::Error>;

    async fn get_message(&self, id: &str) -> Option<Message> {
        let bytes = self.get_message_bytes(id).await.ok()??;
        Message::decode_from_slice(&bytes).ok()
    }

    async fn save_wa_message(
        &self,
        id: &str,
        chat: &str,
        sender: &str,
        msg: &Message,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let bytes = msg.encode_to_vec();
        self.save_message_bytes(id, chat, sender, &bytes).await?;
        Ok(())
    }
}
