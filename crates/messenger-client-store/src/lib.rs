use messenger_crypto::{PrivateIdentity, PublicIdentity};
use messenger_protocol::{Envelope, MessageId, PeerId};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ClientStoreError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("missing identity")]
    MissingIdentity,
}

pub type Result<T> = std::result::Result<T, ClientStoreError>;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum MessageDirection {
    Inbound,
    Outbound,
}

impl MessageDirection {
    fn as_str(self) -> &'static str {
        match self {
            Self::Inbound => "inbound",
            Self::Outbound => "outbound",
        }
    }

    fn from_str(value: &str) -> Self {
        match value {
            "outbound" => Self::Outbound,
            _ => Self::Inbound,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ContactRecord {
    pub peer_id: PeerId,
    pub display_name: String,
    pub public_identity: PublicIdentity,
    pub verified_at_ms: Option<u64>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct MessageRecord {
    pub message_id: MessageId,
    pub conversation_id: String,
    pub peer_id: PeerId,
    pub sender_peer_id: PeerId,
    pub body: String,
    pub created_at_ms: u64,
    pub direction: MessageDirection,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct OutboxRecord {
    pub message_id: MessageId,
    pub envelope: Envelope,
    pub created_at_ms: u64,
    pub retry_count: u32,
}

pub struct ClientStore {
    connection: Connection,
}

impl ClientStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let connection = Connection::open(path)?;
        let store = Self { connection };
        store.migrate()?;
        Ok(store)
    }

    pub fn open_in_memory() -> Result<Self> {
        let connection = Connection::open_in_memory()?;
        let store = Self { connection };
        store.migrate()?;
        Ok(store)
    }

    pub fn save_identity(&self, identity: &PrivateIdentity, peer_id: &PeerId) -> Result<()> {
        self.connection.execute(
            "INSERT OR REPLACE INTO identities (slot, peer_id, private_identity_json, created_at_ms)
             VALUES ('local', ?1, ?2, COALESCE(
                (SELECT created_at_ms FROM identities WHERE slot = 'local'),
                ?3
             ))",
            params![
                peer_id.as_str(),
                serde_json::to_string(identity)?,
                now_ms() as i64
            ],
        )?;
        Ok(())
    }

    pub fn load_identity(&self) -> Result<Option<(PeerId, PrivateIdentity)>> {
        self.connection
            .query_row(
                "SELECT peer_id, private_identity_json
                 FROM identities
                 WHERE slot = 'local'",
                [],
                |row| {
                    let peer_id: String = row.get(0)?;
                    let json: String = row.get(1)?;
                    Ok((peer_id, json))
                },
            )
            .optional()?
            .map(|(peer_id, json)| {
                Ok((
                    PeerId::new(peer_id).map_err(|_| rusqlite::Error::InvalidQuery)?,
                    serde_json::from_str(&json)?,
                ))
            })
            .transpose()
    }

    pub fn upsert_contact(
        &self,
        display_name: &str,
        public_identity: &PublicIdentity,
        verified_at_ms: Option<u64>,
    ) -> Result<()> {
        self.connection.execute(
            "INSERT INTO contacts
                (peer_id, display_name, public_identity_json, verified_at_ms, created_at_ms)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(peer_id) DO UPDATE SET
                display_name = excluded.display_name,
                public_identity_json = excluded.public_identity_json,
                verified_at_ms = excluded.verified_at_ms",
            params![
                public_identity.peer_id.as_str(),
                display_name,
                serde_json::to_string(public_identity)?,
                verified_at_ms.map(|value| value as i64),
                now_ms() as i64,
            ],
        )?;
        Ok(())
    }

    pub fn contacts(&self) -> Result<Vec<ContactRecord>> {
        let mut statement = self.connection.prepare(
            "SELECT peer_id, display_name, public_identity_json, verified_at_ms
             FROM contacts
             ORDER BY display_name ASC, peer_id ASC",
        )?;
        let rows = statement.query_map([], |row| {
            let peer_id: String = row.get(0)?;
            let display_name: String = row.get(1)?;
            let public_identity_json: String = row.get(2)?;
            let verified_at_ms: Option<i64> = row.get(3)?;
            Ok((
                peer_id,
                display_name,
                public_identity_json,
                verified_at_ms.map(|value| value as u64),
            ))
        })?;

        rows.map(|row| {
            let (peer_id, display_name, public_identity_json, verified_at_ms) = row?;
            Ok(ContactRecord {
                peer_id: PeerId::new(peer_id).map_err(|_| rusqlite::Error::InvalidQuery)?,
                display_name,
                public_identity: serde_json::from_str(&public_identity_json)?,
                verified_at_ms,
            })
        })
        .collect()
    }

    pub fn contact_by_name(&self, display_name: &str) -> Result<Option<ContactRecord>> {
        Ok(self
            .contacts()?
            .into_iter()
            .find(|contact| contact.display_name == display_name))
    }

    pub fn save_message(&self, message: &MessageRecord) -> Result<()> {
        self.connection.execute(
            "INSERT OR REPLACE INTO conversations (conversation_id, peer_id, created_at_ms)
             VALUES (?1, ?2, COALESCE(
                (SELECT created_at_ms FROM conversations WHERE conversation_id = ?1),
                ?3
             ))",
            params![
                message.conversation_id,
                message.peer_id.as_str(),
                message.created_at_ms as i64,
            ],
        )?;
        self.connection.execute(
            "INSERT OR REPLACE INTO messages
                (message_id, conversation_id, peer_id, sender_peer_id, body, created_at_ms, direction)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                message.message_id.to_string(),
                message.conversation_id,
                message.peer_id.as_str(),
                message.sender_peer_id.as_str(),
                message.body,
                message.created_at_ms as i64,
                message.direction.as_str(),
            ],
        )?;
        Ok(())
    }

    pub fn messages_for_peer(&self, peer_id: &PeerId) -> Result<Vec<MessageRecord>> {
        let mut statement = self.connection.prepare(
            "SELECT message_id, conversation_id, peer_id, sender_peer_id, body, created_at_ms, direction
             FROM messages
             WHERE peer_id = ?1
             ORDER BY created_at_ms ASC, message_id ASC",
        )?;
        let rows = statement.query_map(params![peer_id.as_str()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, String>(6)?,
            ))
        })?;

        rows.map(|row| {
            let (
                message_id,
                conversation_id,
                peer_id,
                sender_peer_id,
                body,
                created_at_ms,
                direction,
            ) = row?;
            Ok(MessageRecord {
                message_id: MessageId::from_uuid(
                    Uuid::parse_str(&message_id).map_err(|_| rusqlite::Error::InvalidQuery)?,
                ),
                conversation_id,
                peer_id: PeerId::new(peer_id).map_err(|_| rusqlite::Error::InvalidQuery)?,
                sender_peer_id: PeerId::new(sender_peer_id)
                    .map_err(|_| rusqlite::Error::InvalidQuery)?,
                body,
                created_at_ms: created_at_ms as u64,
                direction: MessageDirection::from_str(&direction),
            })
        })
        .collect()
    }

    pub fn enqueue_outbox(&self, envelope: &Envelope) -> Result<()> {
        self.connection.execute(
            "INSERT OR IGNORE INTO outbox (message_id, envelope_json, created_at_ms, retry_count)
             VALUES (?1, ?2, ?3, 0)",
            params![
                envelope.message_id.to_string(),
                serde_json::to_string(envelope)?,
                now_ms() as i64,
            ],
        )?;
        Ok(())
    }

    pub fn outbox(&self) -> Result<Vec<OutboxRecord>> {
        let mut statement = self.connection.prepare(
            "SELECT message_id, envelope_json, created_at_ms, retry_count
             FROM outbox
             ORDER BY created_at_ms ASC, message_id ASC",
        )?;
        let rows = statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, i64>(3)?,
            ))
        })?;

        rows.map(|row| {
            let (message_id, envelope_json, created_at_ms, retry_count) = row?;
            Ok(OutboxRecord {
                message_id: MessageId::from_uuid(
                    Uuid::parse_str(&message_id).map_err(|_| rusqlite::Error::InvalidQuery)?,
                ),
                envelope: serde_json::from_str(&envelope_json)?,
                created_at_ms: created_at_ms as u64,
                retry_count: retry_count as u32,
            })
        })
        .collect()
    }

    pub fn remove_outbox(&self, message_id: &MessageId) -> Result<bool> {
        let removed = self.connection.execute(
            "DELETE FROM outbox WHERE message_id = ?1",
            params![message_id.to_string()],
        )?;
        Ok(removed > 0)
    }

    fn migrate(&self) -> Result<()> {
        self.connection.execute_batch(
            "CREATE TABLE IF NOT EXISTS identities (
                slot TEXT PRIMARY KEY,
                peer_id TEXT NOT NULL,
                private_identity_json TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS contacts (
                peer_id TEXT PRIMARY KEY,
                display_name TEXT NOT NULL,
                public_identity_json TEXT NOT NULL,
                verified_at_ms INTEGER,
                created_at_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS conversations (
                conversation_id TEXT PRIMARY KEY,
                peer_id TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL
            );

            CREATE TABLE IF NOT EXISTS messages (
                message_id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                peer_id TEXT NOT NULL,
                sender_peer_id TEXT NOT NULL,
                body TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL,
                direction TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_messages_peer_created
                ON messages (peer_id, created_at_ms);

            CREATE TABLE IF NOT EXISTS outbox (
                message_id TEXT PRIMARY KEY,
                envelope_json TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL,
                retry_count INTEGER NOT NULL
            );",
        )?;
        Ok(())
    }
}

pub fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use messenger_crypto::IdentityKeypair;

    #[test]
    fn stores_identity_contacts_messages_and_outbox() -> Result<()> {
        let store = ClientStore::open_in_memory()?;
        let alice = IdentityKeypair::generate();
        let bob = IdentityKeypair::generate();
        let bob_public = bob.public_identity();

        store.save_identity(&alice.private_identity(), &alice.peer_id())?;
        let (stored_peer_id, stored_identity) = store
            .load_identity()?
            .ok_or(ClientStoreError::MissingIdentity)?;
        assert_eq!(stored_peer_id, alice.peer_id());
        assert_eq!(stored_identity, alice.private_identity());

        store.upsert_contact("Bob", &bob_public, Some(42))?;
        let contacts = store.contacts()?;
        assert_eq!(contacts.len(), 1);
        assert_eq!(contacts[0].display_name, "Bob");
        assert_eq!(contacts[0].public_identity, bob_public);

        let envelope = alice
            .encrypt_for(&bob_public, b"hello")
            .map_err(|_| rusqlite::Error::InvalidQuery)?;
        store.enqueue_outbox(&envelope)?;
        assert_eq!(store.outbox()?.len(), 1);

        let message = MessageRecord {
            message_id: envelope.message_id,
            conversation_id: bob.peer_id().as_str().to_owned(),
            peer_id: bob.peer_id(),
            sender_peer_id: alice.peer_id(),
            body: "hello".to_owned(),
            created_at_ms: now_ms(),
            direction: MessageDirection::Outbound,
        };
        store.save_message(&message)?;
        let messages = store.messages_for_peer(&message.peer_id)?;
        assert_eq!(messages, vec![message.clone()]);

        assert!(store.remove_outbox(&envelope.message_id)?);
        assert!(store.outbox()?.is_empty());

        Ok(())
    }
}
