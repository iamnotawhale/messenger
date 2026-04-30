use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProtocolVersion(u16);

impl ProtocolVersion {
    pub const CURRENT: Self = Self(PROTOCOL_VERSION);

    pub fn as_u16(self) -> u16 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct PeerId(String);

impl PeerId {
    pub fn new(value: impl Into<String>) -> Result<Self, ProtocolError> {
        let value = value.into();

        if value.len() < 16 || value.len() > 128 {
            return Err(ProtocolError::InvalidPeerId);
        }

        if !value
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || matches!(char, '-' | '_' | ':'))
        {
            return Err(ProtocolError::InvalidPeerId);
        }

        Ok(Self(value))
    }

    pub fn from_public_identity(signing_key: &[u8; 32], agreement_key: &[u8; 32]) -> Self {
        let mut hasher = blake3::Hasher::new();
        hasher.update(b"messenger/peer-id/v1");
        hasher.update(signing_key);
        hasher.update(agreement_key);
        let hash = hasher.finalize();
        Self(format!("peer:{}", &hash.to_hex()[..32]))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl fmt::Display for PeerId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct MessageId(Uuid);

impl MessageId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    pub fn as_bytes(&self) -> &[u8; 16] {
        self.0.as_bytes()
    }
}

impl Default for MessageId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for MessageId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum PayloadKind {
    Text,
    DeliveryReceipt,
    KeyAnnouncement,
    WebRtcSignal,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum TransportKind {
    Relay,
    WebRtc,
    LocalNetwork,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DeliveryPolicy {
    PreferDirect,
    DirectOnly,
    RelayOnly,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ChatMessage {
    pub conversation_id: String,
    pub body: String,
    pub client_created_at_ms: u64,
}

impl ChatMessage {
    pub fn text(conversation_id: impl Into<String>, body: impl Into<String>) -> Self {
        Self {
            conversation_id: conversation_id.into(),
            body: body.into(),
            client_created_at_ms: 0,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PlainMessage {
    pub conversation_id: Uuid,
    pub body: String,
    pub client_created_at_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CipherPayload {
    pub algorithm: String,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Envelope {
    pub version: u16,
    pub message_id: MessageId,
    pub sender: PeerId,
    pub recipient: PeerId,
    pub created_at_ms: u64,
    pub expires_at_ms: Option<u64>,
    pub payload_kind: PayloadKind,
    pub payload: CipherPayload,
    pub signature: Vec<u8>,
}

impl Envelope {
    pub fn new_unsigned(
        sender: PeerId,
        recipient: PeerId,
        created_at_ms: u64,
        payload_kind: PayloadKind,
        payload: CipherPayload,
    ) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            message_id: MessageId::new(),
            sender,
            recipient,
            created_at_ms,
            expires_at_ms: None,
            payload_kind,
            payload,
            signature: Vec::new(),
        }
    }

    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.version.to_be_bytes());
        bytes.extend_from_slice(self.message_id.as_uuid().as_bytes());
        bytes.extend_from_slice(self.sender.as_str().as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(self.recipient.as_str().as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&self.created_at_ms.to_be_bytes());
        bytes.extend_from_slice(&self.expires_at_ms.unwrap_or_default().to_be_bytes());
        bytes.extend_from_slice(format!("{:?}", self.payload_kind).as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(self.payload.algorithm.as_bytes());
        bytes.push(0);
        bytes.extend_from_slice(&self.payload.nonce);
        bytes.push(0);
        bytes.extend_from_slice(&self.payload.ciphertext);
        bytes
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EnvelopeHeader {
    pub version: u16,
    pub message_id: MessageId,
    pub sender: PeerId,
    pub recipient: PeerId,
    pub created_at_ms: u64,
    pub expires_at_ms: Option<u64>,
    pub payload_kind: PayloadKind,
    pub delivery_policy: DeliveryPolicy,
}

impl EnvelopeHeader {
    pub fn new(sender: PeerId, recipient: PeerId) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            message_id: MessageId::new(),
            sender,
            recipient,
            created_at_ms: 0,
            expires_at_ms: None,
            payload_kind: PayloadKind::Text,
            delivery_policy: DeliveryPolicy::PreferDirect,
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("invalid peer id")]
    InvalidPeerId,
    #[error("invalid encrypted envelope")]
    InvalidEnvelope,
    #[error("unsupported protocol version: {0}")]
    UnsupportedVersion(u16),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_short_peer_ids() {
        assert!(PeerId::new("short").is_err());
    }

    #[test]
    fn signing_bytes_change_when_payload_changes() {
        let sender = PeerId::new("peer:aaaaaaaaaaaaaaaa").unwrap();
        let recipient = PeerId::new("peer:bbbbbbbbbbbbbbbb").unwrap();
        let payload = CipherPayload {
            algorithm: "test".to_owned(),
            nonce: vec![1],
            ciphertext: vec![2],
        };
        let mut envelope = Envelope::new_unsigned(sender, recipient, 1, PayloadKind::Text, payload);
        let before = envelope.signing_bytes();
        envelope.payload.ciphertext = vec![3];

        assert_ne!(before, envelope.signing_bytes());
    }
}
