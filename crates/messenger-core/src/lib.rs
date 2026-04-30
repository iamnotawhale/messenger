use messenger_crypto::{CryptoError, IdentityKeypair, PublicIdentity};
use messenger_protocol::{Envelope, MessageId, PeerId, SubmitEnvelopeResponse};
use messenger_storage::{MessageStore, QueueKind, StorageError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("crypto error: {0}")]
    Crypto(#[from] CryptoError),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("transport error: {0}")]
    Transport(String),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DeliveryReceipt {
    pub message_id: MessageId,
    pub accepted: bool,
}

impl From<SubmitEnvelopeResponse> for DeliveryReceipt {
    fn from(response: SubmitEnvelopeResponse) -> Self {
        Self {
            message_id: response.message_id,
            accepted: response.accepted,
        }
    }
}

#[async_trait::async_trait]
pub trait MessageTransport: Send + Sync {
    async fn send(&self, envelope: Envelope) -> Result<DeliveryReceipt, CoreError>;

    async fn pending(&self, limit: usize) -> Result<Vec<Envelope>, CoreError>;

    async fn mark_delivered(&self, message_id: MessageId) -> Result<(), CoreError>;
}

#[derive(Clone)]
pub struct MessengerCore {
    identity: IdentityKeypair,
}

impl MessengerCore {
    pub fn new(identity: IdentityKeypair) -> Self {
        Self { identity }
    }

    pub fn generate() -> Self {
        Self::new(IdentityKeypair::generate())
    }

    pub fn peer_id(&self) -> PeerId {
        self.identity.peer_id()
    }

    pub fn identity(&self) -> &IdentityKeypair {
        &self.identity
    }

    pub fn compose_message(
        &self,
        recipient: &PublicIdentity,
        plaintext: &[u8],
    ) -> Result<Envelope, CoreError> {
        Ok(self.identity.encrypt_for(recipient, plaintext)?)
    }

    pub fn open_message(
        &self,
        sender: &PublicIdentity,
        envelope: &Envelope,
    ) -> Result<Vec<u8>, CoreError> {
        Ok(self.identity.decrypt_from(sender, envelope)?)
    }

    pub async fn persist_inbound<S: MessageStore + Sync>(
        &self,
        store: &S,
        envelope: Envelope,
    ) -> Result<(), CoreError> {
        store.enqueue(QueueKind::Inbox, envelope).await?;
        Ok(())
    }

    pub async fn send_with<T: MessageTransport>(
        &self,
        transport: &T,
        envelope: Envelope,
    ) -> Result<DeliveryReceipt, CoreError> {
        transport.send(envelope).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_core_has_peer_id() {
        let core = MessengerCore::generate();
        assert!(!core.peer_id().as_str().is_empty());
    }
}
