use messenger_client::{ClientError, MessengerClient};
use messenger_client_store::{ClientStoreError, ContactRecord, MessageDirection, MessageRecord};
use messenger_crypto::PublicIdentity;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum FfiError {
    #[error("client error: {0}")]
    Client(#[from] ClientError),
    #[error("store error: {0}")]
    Store(#[from] ClientStoreError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, FfiError>;

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ClientConfig {
    pub database_path: String,
    pub relay_url: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ContactDto {
    pub name: String,
    pub peer_id: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct MessageDto {
    pub message_id: String,
    pub contact_name: String,
    pub direction: String,
    pub body: String,
    pub created_at_ms: u64,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SyncedMessageDto {
    pub message_id: String,
    pub sender_peer_id: String,
    pub body: String,
}

pub fn init_client(config: ClientConfig) -> Result<String> {
    let client = open_client(&config)?;
    Ok(client.init_identity()?.to_string())
}

pub fn export_public_identity(config: ClientConfig) -> Result<String> {
    let client = open_client(&config)?;
    let identity = client.identity()?;
    Ok(serde_json::to_string_pretty(&identity.public_identity())?)
}

pub fn add_contact(config: ClientConfig, name: String, public_identity_json: String) -> Result<()> {
    let public_identity: PublicIdentity = serde_json::from_str(&public_identity_json)?;
    open_client(&config)?.add_contact(name, &public_identity)?;
    Ok(())
}

pub fn list_contacts(config: ClientConfig) -> Result<Vec<ContactDto>> {
    let client = open_client(&config)?;
    client
        .store()
        .contacts()?
        .into_iter()
        .map(contact_to_dto)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(FfiError::from)
}

pub fn send_message(config: ClientConfig, contact_name: String, body: String) -> Result<String> {
    let client = open_client(&config)?;
    let sent = client.send_message(&contact_name, &body)?;
    Ok(sent.message_id.to_string())
}

pub fn sync(config: ClientConfig) -> Result<Vec<SyncedMessageDto>> {
    let client = open_client(&config)?;
    Ok(client
        .sync_pending()?
        .into_iter()
        .map(|message| SyncedMessageDto {
            message_id: message.message_id.to_string(),
            sender_peer_id: message.sender.to_string(),
            body: message.body,
        })
        .collect())
}

pub fn list_messages(config: ClientConfig, contact_name: String) -> Result<Vec<MessageDto>> {
    let client = open_client(&config)?;
    let contact = client
        .store()
        .contact_by_name(&contact_name)?
        .ok_or_else(|| ClientError::UnknownContact(contact_name.clone()))?;
    Ok(client
        .store()
        .messages_for_peer(&contact.peer_id)?
        .into_iter()
        .map(|message| message_to_dto(contact.display_name.clone(), message))
        .collect())
}

fn open_client(config: &ClientConfig) -> Result<MessengerClient> {
    Ok(MessengerClient::open(
        &config.database_path,
        config.relay_url.clone(),
    )?)
}

fn contact_to_dto(contact: ContactRecord) -> std::result::Result<ContactDto, FfiError> {
    Ok(ContactDto {
        name: contact.display_name,
        peer_id: contact.peer_id.to_string(),
    })
}

fn message_to_dto(contact_name: String, message: MessageRecord) -> MessageDto {
    let direction = match message.direction {
        MessageDirection::Inbound => "inbound",
        MessageDirection::Outbound => "outbound",
    };
    MessageDto {
        message_id: message.message_id.to_string(),
        contact_name,
        direction: direction.to_owned(),
        body: message.body,
        created_at_ms: message.created_at_ms,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_client_is_idempotent() -> Result<()> {
        let database_path = std::env::temp_dir().join(format!(
            "messenger-ffi-{}-{}.db",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default()
        ));
        let config = ClientConfig {
            database_path: database_path.to_string_lossy().to_string(),
            relay_url: "http://127.0.0.1:8080".to_owned(),
        };

        let first = init_client(config.clone())?;
        let second = init_client(config)?;
        let _ = std::fs::remove_file(database_path);

        assert_eq!(first, second);
        Ok(())
    }
}
