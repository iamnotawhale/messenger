//! Cryptographic primitives for the messenger core.
//!
//! This crate intentionally exposes a small, opinionated API: identity keys,
//! contact public keys, and encrypted signed envelopes. Higher-level ratchets
//! can be layered on top without changing the transport or storage crates.

use chacha20poly1305::{
    aead::{Aead, KeyInit},
    Key, XChaCha20Poly1305, XNonce,
};
use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use messenger_protocol::{
    AuthChallenge, CipherPayload, Envelope, PayloadKind, PeerId, PlainMessage, ProtocolError,
    PROTOCOL_VERSION,
};
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use x25519_dalek::{PublicKey as X25519PublicKey, StaticSecret};

const NONCE_LEN: usize = 24;

#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("invalid public key length")]
    InvalidPublicKeyLength,
    #[error("invalid private key length")]
    InvalidPrivateKeyLength,
    #[error("invalid signature")]
    InvalidSignature,
    #[error("encryption failed")]
    EncryptionFailed,
    #[error("decryption failed")]
    DecryptionFailed,
    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PublicIdentity {
    pub peer_id: PeerId,
    pub signing_key: [u8; 32],
    pub agreement_key: [u8; 32],
}

impl PublicIdentity {
    pub fn from_keys(signing_key: VerifyingKey, agreement_key: X25519PublicKey) -> Self {
        let signing_key = signing_key.to_bytes();
        let agreement_key = agreement_key.to_bytes();
        let peer_id = PeerId::from_public_identity(&signing_key, &agreement_key);

        Self {
            peer_id,
            signing_key,
            agreement_key,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct PrivateIdentity {
    pub signing_key: [u8; 32],
    pub agreement_secret: [u8; 32],
}

#[derive(Clone)]
pub struct IdentityKeypair {
    signing_key: SigningKey,
    agreement_secret: StaticSecret,
}

impl IdentityKeypair {
    pub fn generate() -> Self {
        Self {
            signing_key: SigningKey::generate(&mut OsRng),
            agreement_secret: StaticSecret::random_from_rng(OsRng),
        }
    }

    pub fn from_private_identity(identity: PrivateIdentity) -> Self {
        Self {
            signing_key: SigningKey::from_bytes(&identity.signing_key),
            agreement_secret: StaticSecret::from(identity.agreement_secret),
        }
    }

    pub fn private_identity(&self) -> PrivateIdentity {
        PrivateIdentity {
            signing_key: self.signing_key.to_bytes(),
            agreement_secret: self.agreement_secret.to_bytes(),
        }
    }

    pub fn public_identity(&self) -> PublicIdentity {
        PublicIdentity::from_keys(
            self.signing_key.verifying_key(),
            self.agreement_public_key(),
        )
    }

    pub fn peer_id(&self) -> PeerId {
        self.public_identity().peer_id
    }

    pub fn agreement_public_key(&self) -> X25519PublicKey {
        X25519PublicKey::from(&self.agreement_secret)
    }

    pub fn sign_auth_challenge(&self, challenge: &AuthChallenge) -> Vec<u8> {
        self.signing_key
            .sign(&challenge.signing_bytes())
            .to_bytes()
            .to_vec()
    }

    pub fn encrypt_for(
        &self,
        recipient: &PublicIdentity,
        plaintext: &[u8],
    ) -> Result<Envelope, CryptoError> {
        let sender = self.public_identity();
        let nonce = random_nonce();
        let payload = CipherPayload {
            algorithm: "xchacha20poly1305+blake3-x25519".to_owned(),
            nonce: nonce.to_vec(),
            ciphertext: Vec::new(),
        };
        let mut envelope = Envelope::new_unsigned(
            sender.peer_id,
            recipient.peer_id.clone(),
            current_time_ms(),
            PayloadKind::Text,
            payload,
        );
        let key = derive_message_key(
            &self.agreement_secret,
            &x25519_from_bytes(recipient.agreement_key)?,
            &nonce,
            &envelope,
        );
        let ciphertext = XChaCha20Poly1305::new(Key::from_slice(&key))
            .encrypt(XNonce::from_slice(&nonce), plaintext)
            .map_err(|_| CryptoError::EncryptionFailed)?;
        envelope.payload.ciphertext = ciphertext;
        envelope.signature = self.sign_envelope(&envelope);

        Ok(envelope)
    }

    pub fn decrypt_from(
        &self,
        sender: &PublicIdentity,
        envelope: &Envelope,
    ) -> Result<Vec<u8>, CryptoError> {
        if envelope.sender != sender.peer_id {
            return Err(CryptoError::InvalidSignature);
        }

        verify_envelope_signature(sender, envelope)?;
        let nonce: [u8; NONCE_LEN] = envelope
            .payload
            .nonce
            .as_slice()
            .try_into()
            .map_err(|_| CryptoError::DecryptionFailed)?;

        let key = derive_message_key(
            &self.agreement_secret,
            &x25519_from_bytes(sender.agreement_key)?,
            &nonce,
            envelope,
        );

        XChaCha20Poly1305::new(Key::from_slice(&key))
            .decrypt(
                XNonce::from_slice(&nonce),
                envelope.payload.ciphertext.as_slice(),
            )
            .map_err(|_| CryptoError::DecryptionFailed)
    }

    fn sign_envelope(&self, envelope: &Envelope) -> Vec<u8> {
        let message = signature_message(envelope);
        self.signing_key.sign(&message).to_bytes().to_vec()
    }
}

pub fn verify_envelope_signature(
    sender: &PublicIdentity,
    envelope: &Envelope,
) -> Result<(), CryptoError> {
    let verifying_key = VerifyingKey::from_bytes(&sender.signing_key)
        .map_err(|_| CryptoError::InvalidPublicKeyLength)?;
    let signature =
        Signature::from_slice(&envelope.signature).map_err(|_| CryptoError::InvalidSignature)?;
    verifying_key
        .verify(&signature_message(envelope), &signature)
        .map_err(|_| CryptoError::InvalidSignature)
}

pub fn verify_auth_challenge(
    sender: &PublicIdentity,
    challenge: &AuthChallenge,
    signature: &[u8],
) -> Result<(), CryptoError> {
    let verifying_key = VerifyingKey::from_bytes(&sender.signing_key)
        .map_err(|_| CryptoError::InvalidPublicKeyLength)?;
    let signature = Signature::from_slice(signature).map_err(|_| CryptoError::InvalidSignature)?;
    verifying_key
        .verify(&challenge.signing_bytes(), &signature)
        .map_err(|_| CryptoError::InvalidSignature)
}

fn x25519_from_bytes(bytes: [u8; 32]) -> Result<X25519PublicKey, CryptoError> {
    Ok(X25519PublicKey::from(bytes))
}

fn random_nonce() -> [u8; NONCE_LEN] {
    let mut nonce = [0_u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce);
    nonce
}

fn derive_message_key(
    own_secret: &StaticSecret,
    peer_public: &X25519PublicKey,
    nonce: &[u8; NONCE_LEN],
    envelope: &Envelope,
) -> [u8; 32] {
    let shared_secret = own_secret.diffie_hellman(peer_public);
    let mut hasher = blake3::Hasher::new_derive_key("messenger/v1/p2p-envelope");
    hasher.update(shared_secret.as_bytes());
    hasher.update(nonce);
    hasher.update(envelope.message_id.as_uuid().as_bytes());
    hasher.update(envelope.sender.as_str().as_bytes());
    hasher.update(envelope.recipient.as_str().as_bytes());
    *hasher.finalize().as_bytes()
}

fn signature_message(envelope: &Envelope) -> Vec<u8> {
    let mut message = Vec::with_capacity(4 + envelope.signing_bytes().len());
    message.extend_from_slice(&PROTOCOL_VERSION.to_be_bytes());
    message.extend_from_slice(&envelope.signing_bytes());
    message
}

fn current_time_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

pub struct SealedMessage;

impl SealedMessage {
    pub fn seal_text(
        sender: &IdentityKeypair,
        recipient: &PublicIdentity,
        message: &PlainMessage,
    ) -> Result<Envelope, CryptoError> {
        let plaintext = format!(
            "{}\0{}\0{}",
            message.conversation_id, message.client_created_at_ms, message.body
        );
        sender.encrypt_for(recipient, plaintext.as_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypted_envelope_round_trips() {
        let alice = IdentityKeypair::generate();
        let bob = IdentityKeypair::generate();
        let plaintext = b"hello bob";

        let envelope = alice
            .encrypt_for(&bob.public_identity(), plaintext)
            .unwrap();
        let decrypted = bob
            .decrypt_from(&alice.public_identity(), &envelope)
            .unwrap();

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn tampered_ciphertext_fails_signature_verification() {
        let alice = IdentityKeypair::generate();
        let bob = IdentityKeypair::generate();
        let mut envelope = alice
            .encrypt_for(&bob.public_identity(), b"hello bob")
            .unwrap();

        envelope.payload.ciphertext[0] ^= 1;

        let result = bob.decrypt_from(&alice.public_identity(), &envelope);
        assert!(matches!(result, Err(CryptoError::InvalidSignature)));
    }

    #[test]
    fn auth_challenge_signature_verifies() {
        let alice = IdentityKeypair::generate();
        let challenge = AuthChallenge {
            challenge_id: "challenge-1".to_owned(),
            peer_id: alice.peer_id(),
            nonce: "nonce".to_owned(),
            expires_at_ms: 1,
        };
        let signature = alice.sign_auth_challenge(&challenge);

        verify_auth_challenge(&alice.public_identity(), &challenge, &signature).unwrap();
    }

    #[test]
    fn auth_challenge_signature_rejects_wrong_challenge() {
        let alice = IdentityKeypair::generate();
        let challenge = AuthChallenge {
            challenge_id: "challenge-1".to_owned(),
            peer_id: alice.peer_id(),
            nonce: "nonce".to_owned(),
            expires_at_ms: 1,
        };
        let mut other = challenge.clone();
        other.nonce = "other".to_owned();
        let signature = alice.sign_auth_challenge(&challenge);

        let result = verify_auth_challenge(&alice.public_identity(), &other, &signature);

        assert!(matches!(result, Err(CryptoError::InvalidSignature)));
    }
}
