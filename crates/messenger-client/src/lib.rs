use messenger_client_store::{
    ClientStore, ClientStoreError, MessageDirection, MessageRecord, OutboxRecord,
};
use messenger_crypto::{CryptoError, IdentityKeypair, PrivateIdentity, PublicIdentity};
use messenger_protocol::{MessageId, PeerId};
use messenger_transport::{RelayHttpClient, TransportError};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ClientError {
    #[error("store error: {0}")]
    Store(#[from] ClientStoreError),
    #[error("crypto error: {0}")]
    Crypto(#[from] CryptoError),
    #[error("transport error: {0}")]
    Transport(#[from] TransportError),
    #[error("missing local identity")]
    MissingIdentity,
    #[error("unknown contact: {0}")]
    UnknownContact(String),
}

pub type Result<T> = std::result::Result<T, ClientError>;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SentMessage {
    pub message_id: MessageId,
    pub accepted: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SyncedMessage {
    pub message_id: MessageId,
    pub sender: PeerId,
    pub body: String,
}

pub struct MessengerClient {
    store: ClientStore,
    transport: RelayHttpClient,
}

impl MessengerClient {
    pub fn open(database_path: impl AsRef<Path>, relay_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            store: ClientStore::open(database_path)?,
            transport: RelayHttpClient::new(relay_url)?,
        })
    }

    pub fn open_in_memory(relay_url: impl Into<String>) -> Result<Self> {
        Ok(Self {
            store: ClientStore::open_in_memory()?,
            transport: RelayHttpClient::new(relay_url)?,
        })
    }

    pub fn store(&self) -> &ClientStore {
        &self.store
    }

    pub fn init_identity(&self) -> Result<PeerId> {
        if let Some((peer_id, _)) = self.store.load_identity()? {
            return Ok(peer_id);
        }

        let identity = IdentityKeypair::generate();
        let peer_id = identity.peer_id();
        self.store
            .save_identity(&identity.private_identity(), &peer_id)?;
        Ok(peer_id)
    }

    pub fn import_identity(&self, identity: PrivateIdentity) -> Result<PeerId> {
        let keypair = IdentityKeypair::from_private_identity(identity);
        let peer_id = keypair.peer_id();
        self.store
            .save_identity(&keypair.private_identity(), &peer_id)?;
        Ok(peer_id)
    }

    pub fn identity(&self) -> Result<IdentityKeypair> {
        let (_, identity) = self
            .store
            .load_identity()?
            .ok_or(ClientError::MissingIdentity)?;
        Ok(IdentityKeypair::from_private_identity(identity))
    }

    pub fn add_contact(
        &self,
        display_name: impl AsRef<str>,
        public_identity: &PublicIdentity,
    ) -> Result<()> {
        self.store
            .upsert_contact(display_name.as_ref(), public_identity, None)?;
        Ok(())
    }

    pub fn send_message(&self, contact_name: &str, body: &str) -> Result<SentMessage> {
        let identity = self.identity()?;
        let contact = self
            .store
            .contact_by_name(contact_name)?
            .ok_or_else(|| ClientError::UnknownContact(contact_name.to_owned()))?;
        let envelope = identity.encrypt_for(&contact.public_identity, body.as_bytes())?;
        let message_id = envelope.message_id;
        let session = self.transport.authenticate(&identity)?;

        self.store.enqueue_outbox(&envelope)?;
        let response = self.transport.submit(&session, envelope)?;
        if response.accepted {
            self.store.remove_outbox(&message_id)?;
        }

        self.store.save_message(&MessageRecord {
            message_id,
            conversation_id: contact.peer_id.as_str().to_owned(),
            peer_id: contact.peer_id,
            sender_peer_id: identity.peer_id(),
            body: body.to_owned(),
            created_at_ms: now_ms(),
            direction: MessageDirection::Outbound,
        })?;

        Ok(SentMessage {
            message_id,
            accepted: response.accepted,
        })
    }

    pub fn sync_pending(&self) -> Result<Vec<SyncedMessage>> {
        let identity = self.identity()?;
        let session = self.transport.authenticate(&identity)?;
        let envelopes = self.transport.pending(&session)?;
        let contacts = self.store.contacts()?;
        let mut synced = Vec::new();

        for envelope in envelopes {
            let Some(contact) = contacts
                .iter()
                .find(|contact| contact.peer_id == envelope.sender)
            else {
                continue;
            };

            let plaintext = identity.decrypt_from(&contact.public_identity, &envelope)?;
            let body = String::from_utf8_lossy(&plaintext).to_string();
            self.store.save_message(&MessageRecord {
                message_id: envelope.message_id,
                conversation_id: contact.peer_id.as_str().to_owned(),
                peer_id: contact.peer_id.clone(),
                sender_peer_id: contact.peer_id.clone(),
                body: body.clone(),
                created_at_ms: now_ms(),
                direction: MessageDirection::Inbound,
            })?;
            self.transport
                .mark_delivered(&session, envelope.message_id)?;
            synced.push(SyncedMessage {
                message_id: envelope.message_id,
                sender: contact.peer_id.clone(),
                body,
            });
        }

        Ok(synced)
    }

    pub fn outbox(&self) -> Result<Vec<OutboxRecord>> {
        Ok(self.store.outbox()?)
    }

    pub fn messages_for_contact(&self, contact_name: &str) -> Result<Vec<MessageRecord>> {
        let contact = self
            .store
            .contact_by_name(contact_name)?
            .ok_or_else(|| ClientError::UnknownContact(contact_name.to_owned()))?;
        Ok(self.store.messages_for_peer(&contact.peer_id)?)
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_identity_is_idempotent() -> Result<()> {
        let client = MessengerClient::open_in_memory("http://127.0.0.1:8080")?;
        let first = client.init_identity()?;
        let second = client.init_identity()?;

        assert_eq!(first, second);
        Ok(())
    }
}
