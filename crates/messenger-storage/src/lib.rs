use async_trait::async_trait;
use messenger_protocol::{Envelope, MessageId, PeerId};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("message not found: {0:?}")]
    NotFound(MessageId),
    #[error("storage backend error: {0}")]
    Backend(String),
}

pub type Result<T> = std::result::Result<T, StorageError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueueKind {
    Inbox,
    Outbox,
}

#[async_trait]
pub trait MessageStore: Send + Sync {
    async fn enqueue(&self, kind: QueueKind, envelope: Envelope) -> Result<()>;

    async fn pending_for_peer(&self, peer_id: &PeerId, limit: usize) -> Result<Vec<Envelope>>;

    async fn mark_delivered(&self, message_id: &MessageId) -> Result<()>;
}
