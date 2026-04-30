use messenger_crypto::{CryptoError, IdentityKeypair, PublicIdentity};
use messenger_protocol::{Envelope, PeerId};
use messenger_storage::{MessageStore, QueueKind, StorageError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("crypto error: {0}")]
    Crypto(#[from] CryptoError),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
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
