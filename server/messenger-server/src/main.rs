use async_trait::async_trait;
use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use messenger_crypto::{verify_auth_challenge, PublicIdentity};
use messenger_protocol::{
    AuthChallenge, AuthChallengeRequest, AuthVerifyRequest, AuthVerifyResponse, Envelope,
    MarkDeliveredResponse, PendingEnvelopesResponse, PublicIdentityDocument, SubmitEnvelopeRequest,
    SubmitEnvelopeResponse, TransportKind, PROTOCOL_VERSION,
};
use rand_core::{OsRng, RngCore};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use thiserror::Error;
use uuid::Uuid;

const AUTH_HEADER: &str = "authorization";
const CHALLENGE_TTL: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    protocol_version: u16,
    transports: [TransportKind; 2],
}

#[derive(Clone)]
struct AppState {
    challenges: Arc<Mutex<HashMap<String, PendingChallenge>>>,
    sessions: Arc<Mutex<HashMap<String, PublicIdentity>>>,
    relay_store: Arc<dyn RelayStore>,
}

#[derive(Debug, Clone)]
struct PendingChallenge {
    challenge: AuthChallenge,
}

#[derive(Debug, Serialize, Deserialize)]
struct ErrorResponse {
    error: &'static str,
}

#[derive(Debug, Error)]
enum RelayStoreError {
    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("store lock poisoned")]
    LockPoisoned,
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[async_trait]
trait RelayStore: Send + Sync {
    async fn enqueue(&self, envelope: Envelope) -> Result<(), RelayStoreError>;

    async fn pending_for(&self, recipient: &str) -> Result<Vec<Envelope>, RelayStoreError>;

    async fn mark_delivered(
        &self,
        recipient: &str,
        message_id: Uuid,
    ) -> Result<bool, RelayStoreError>;
}

#[derive(Default)]
struct MemoryRelayStore {
    queues: Mutex<HashMap<String, Vec<Envelope>>>,
}

#[async_trait]
impl RelayStore for MemoryRelayStore {
    async fn enqueue(&self, envelope: Envelope) -> Result<(), RelayStoreError> {
        let recipient = envelope.recipient.as_str().to_owned();
        self.queues
            .lock()
            .map_err(|_| RelayStoreError::LockPoisoned)?
            .entry(recipient)
            .or_default()
            .push(envelope);
        Ok(())
    }

    async fn pending_for(&self, recipient: &str) -> Result<Vec<Envelope>, RelayStoreError> {
        Ok(self
            .queues
            .lock()
            .map_err(|_| RelayStoreError::LockPoisoned)?
            .get(recipient)
            .cloned()
            .unwrap_or_default())
    }

    async fn mark_delivered(
        &self,
        recipient: &str,
        message_id: Uuid,
    ) -> Result<bool, RelayStoreError> {
        let mut queues = self
            .queues
            .lock()
            .map_err(|_| RelayStoreError::LockPoisoned)?;
        let queue = queues.entry(recipient.to_owned()).or_default();
        let before = queue.len();
        queue.retain(|envelope| envelope.message_id.as_uuid() != message_id);
        Ok(before != queue.len())
    }
}

#[derive(Clone)]
struct SqliteRelayStore {
    connection: Arc<Mutex<Connection>>,
}

impl SqliteRelayStore {
    fn connect(database_path: &str) -> Result<Self, RelayStoreError> {
        let connection = Connection::open(database_path)?;
        let store = Self {
            connection: Arc::new(Mutex::new(connection)),
        };
        store.migrate()?;
        Ok(store)
    }

    fn migrate(&self) -> Result<(), RelayStoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| RelayStoreError::LockPoisoned)?;
        connection.execute(
            "CREATE TABLE IF NOT EXISTS relay_envelopes (
                message_id TEXT NOT NULL,
                recipient_peer_id TEXT NOT NULL,
                envelope_json TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL,
                PRIMARY KEY (recipient_peer_id, message_id)
            )",
            [],
        )?;

        connection.execute(
            "CREATE INDEX IF NOT EXISTS idx_relay_envelopes_recipient_created
                ON relay_envelopes (recipient_peer_id, created_at_ms)",
            [],
        )?;

        Ok(())
    }
}

#[async_trait]
impl RelayStore for SqliteRelayStore {
    async fn enqueue(&self, envelope: Envelope) -> Result<(), RelayStoreError> {
        let envelope_json = serde_json::to_string(&envelope)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| RelayStoreError::LockPoisoned)?;
        connection.execute(
            "INSERT OR IGNORE INTO relay_envelopes
                (message_id, recipient_peer_id, envelope_json, created_at_ms)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                envelope.message_id.to_string(),
                envelope.recipient.as_str(),
                envelope_json,
                envelope.created_at_ms as i64
            ],
        )?;

        Ok(())
    }

    async fn pending_for(&self, recipient: &str) -> Result<Vec<Envelope>, RelayStoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| RelayStoreError::LockPoisoned)?;
        let mut statement = connection.prepare(
            "SELECT envelope_json
             FROM relay_envelopes
             WHERE recipient_peer_id = ?1
             ORDER BY created_at_ms ASC, message_id ASC",
        )?;
        let rows = statement.query_map(params![recipient], |row| row.get::<_, String>(0))?;

        rows.map(|row| {
            let json = row?;
            serde_json::from_str(&json).map_err(RelayStoreError::from)
        })
        .collect()
    }

    async fn mark_delivered(
        &self,
        recipient: &str,
        message_id: Uuid,
    ) -> Result<bool, RelayStoreError> {
        let connection = self
            .connection
            .lock()
            .map_err(|_| RelayStoreError::LockPoisoned)?;
        let rows_affected = connection.execute(
            "DELETE FROM relay_envelopes
             WHERE recipient_peer_id = ?1 AND message_id = ?2",
            params![recipient, message_id.to_string()],
        )?;

        Ok(rows_affected > 0)
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            challenges: Arc::new(Mutex::new(HashMap::new())),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            relay_store: Arc::new(MemoryRelayStore::default()),
        }
    }
}

impl AppState {
    fn with_relay_store(relay_store: Arc<dyn RelayStore>) -> Self {
        Self {
            relay_store,
            ..Self::default()
        }
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = match std::env::var("MESSENGER_SQLITE_PATH") {
        Ok(database_path) => {
            let relay_store = SqliteRelayStore::connect(&database_path)
                .unwrap_or_else(|error| panic!("failed to initialize sqlite relay store: {error}"));
            AppState::with_relay_store(Arc::new(relay_store))
        }
        Err(_) => AppState::default(),
    };
    let app = build_router(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap_or_else(|error| panic!("failed to bind server: {error}"));

    axum::serve(listener, app)
        .await
        .unwrap_or_else(|error| panic!("server failed: {error}"));
}

fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/auth/challenge", post(create_challenge))
        .route("/v1/auth/verify", post(verify_challenge))
        .route("/v1/relay/envelopes", post(submit_envelope))
        .route("/v1/relay/envelopes/pending", get(pending_envelopes))
        .route(
            "/v1/relay/envelopes/{message_id}/delivered",
            post(mark_delivered),
        )
        .route("/v1/signaling", get(signaling_placeholder))
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        protocol_version: PROTOCOL_VERSION,
        transports: [TransportKind::Relay, TransportKind::WebRtc],
    })
}

async fn signaling_placeholder() -> &'static str {
    "signaling websocket endpoint placeholder"
}

async fn create_challenge(
    State(state): State<AppState>,
    Json(request): Json<AuthChallengeRequest>,
) -> Result<Json<AuthChallenge>, (StatusCode, Json<ErrorResponse>)> {
    let mut nonce = [0_u8; 32];
    OsRng.fill_bytes(&mut nonce);
    let challenge_id = Uuid::new_v4().to_string();
    let expires_at_ms = now_ms() + CHALLENGE_TTL.as_millis() as u64;
    let challenge = AuthChallenge {
        challenge_id: challenge_id.clone(),
        peer_id: request.peer_id,
        nonce: hex_encode(&nonce),
        expires_at_ms,
    };
    let pending = PendingChallenge {
        challenge: challenge.clone(),
    };

    state
        .challenges
        .lock()
        .map_err(|_| internal_error())?
        .insert(challenge_id.clone(), pending);

    Ok(Json(challenge))
}

async fn verify_challenge(
    State(state): State<AppState>,
    Json(request): Json<AuthVerifyRequest>,
) -> Result<Json<AuthVerifyResponse>, (StatusCode, Json<ErrorResponse>)> {
    let pending = state
        .challenges
        .lock()
        .map_err(|_| internal_error())?
        .remove(&request.challenge_id)
        .ok_or_else(|| unauthorized("unknown challenge"))?;

    if pending.challenge.expires_at_ms < now_ms() {
        return Err(unauthorized("expired challenge"));
    }

    if pending.challenge.peer_id != request.identity.peer_id {
        return Err(unauthorized("challenge peer mismatch"));
    }

    request
        .identity
        .validate_peer_id()
        .map_err(|_| unauthorized("identity peer mismatch"))?;
    let identity = public_identity_from_document(request.identity);

    verify_auth_challenge(&identity, &pending.challenge, &request.signature)
        .map_err(|_| unauthorized("invalid signature"))?;

    let session_token = Uuid::new_v4().to_string();
    state
        .sessions
        .lock()
        .map_err(|_| internal_error())?
        .insert(session_token.clone(), identity.clone());

    Ok(Json(AuthVerifyResponse {
        peer_id: identity.peer_id,
        session_token,
        expires_at_ms: now_ms() + Duration::from_secs(24 * 60 * 60).as_millis() as u64,
    }))
}

async fn submit_envelope(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<SubmitEnvelopeRequest>,
) -> Result<Json<SubmitEnvelopeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let identity = authenticate(&state, &headers)?;

    if request.envelope.sender != identity.peer_id {
        return Err(unauthorized("sender does not match session"));
    }

    let message_id = request.envelope.message_id;
    state
        .relay_store
        .enqueue(request.envelope)
        .await
        .map_err(|_| internal_error())?;

    Ok(Json(SubmitEnvelopeResponse {
        message_id,
        accepted: true,
    }))
}

async fn pending_envelopes(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<PendingEnvelopesResponse>, (StatusCode, Json<ErrorResponse>)> {
    let identity = authenticate(&state, &headers)?;
    let envelopes = state
        .relay_store
        .pending_for(identity.peer_id.as_str())
        .await
        .map_err(|_| internal_error())?;

    Ok(Json(PendingEnvelopesResponse { envelopes }))
}

async fn mark_delivered(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(message_id): Path<Uuid>,
) -> Result<Json<MarkDeliveredResponse>, (StatusCode, Json<ErrorResponse>)> {
    let identity = authenticate(&state, &headers)?;
    let removed = state
        .relay_store
        .mark_delivered(identity.peer_id.as_str(), message_id)
        .await
        .map_err(|_| internal_error())?;

    Ok(Json(MarkDeliveredResponse { removed }))
}

fn authenticate(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<PublicIdentity, (StatusCode, Json<ErrorResponse>)> {
    let header = headers
        .get(AUTH_HEADER)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| unauthorized("missing authorization"))?;
    let token = header
        .strip_prefix("Bearer ")
        .ok_or_else(|| unauthorized("invalid authorization scheme"))?;

    state
        .sessions
        .lock()
        .map_err(|_| internal_error())?
        .get(token)
        .cloned()
        .ok_or_else(|| unauthorized("invalid session"))
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or_default()
}

fn unauthorized(error: &'static str) -> (StatusCode, Json<ErrorResponse>) {
    (StatusCode::UNAUTHORIZED, Json(ErrorResponse { error }))
}

fn internal_error() -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: "internal server error",
        }),
    )
}

fn public_identity_from_document(document: PublicIdentityDocument) -> PublicIdentity {
    PublicIdentity {
        peer_id: document.peer_id,
        signing_key: document.signing_key,
        agreement_key: document.agreement_key,
    }
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{header, request::Builder, Method, Request},
    };
    use messenger_crypto::IdentityKeypair;
    use messenger_protocol::{
        AuthChallengeRequest, AuthVerifyRequest, AuthVerifyResponse, MarkDeliveredResponse,
        PendingEnvelopesResponse, PublicIdentityDocument, SubmitEnvelopeRequest,
        SubmitEnvelopeResponse,
    };
    use serde::{de::DeserializeOwned, Serialize};
    use tower::ServiceExt;

    #[tokio::test]
    async fn relay_delivers_encrypted_envelope_end_to_end() {
        let app = build_router(AppState::default());
        let alice = IdentityKeypair::generate();
        let bob = IdentityKeypair::generate();

        let alice_session = authenticate_peer(app.clone(), &alice).await;
        let bob_session = authenticate_peer(app.clone(), &bob).await;
        let plaintext = b"hello through relay";
        let envelope = alice
            .encrypt_for(&bob.public_identity(), plaintext)
            .unwrap();
        let message_id = envelope.message_id;

        let submit: SubmitEnvelopeResponse = request_json(
            app.clone(),
            Request::builder()
                .method(Method::POST)
                .uri("/v1/relay/envelopes")
                .header(header::AUTHORIZATION, bearer(&alice_session)),
            &SubmitEnvelopeRequest { envelope },
        )
        .await;
        assert!(submit.accepted);
        assert_eq!(submit.message_id, message_id);

        let pending: PendingEnvelopesResponse = request_empty(
            app.clone(),
            Request::builder()
                .method(Method::GET)
                .uri("/v1/relay/envelopes/pending")
                .header(header::AUTHORIZATION, bearer(&bob_session)),
        )
        .await;
        assert_eq!(pending.envelopes.len(), 1);

        let decrypted = bob
            .decrypt_from(&alice.public_identity(), &pending.envelopes[0])
            .unwrap();
        assert_eq!(decrypted, plaintext);

        let delivered: MarkDeliveredResponse = request_empty(
            app.clone(),
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/v1/relay/envelopes/{}/delivered",
                    message_id.as_uuid()
                ))
                .header(header::AUTHORIZATION, bearer(&bob_session)),
        )
        .await;
        assert!(delivered.removed);

        let after_delivery: PendingEnvelopesResponse = request_empty(
            app,
            Request::builder()
                .method(Method::GET)
                .uri("/v1/relay/envelopes/pending")
                .header(header::AUTHORIZATION, bearer(&bob_session)),
        )
        .await;
        assert!(after_delivery.envelopes.is_empty());
    }

    #[tokio::test]
    async fn sqlite_relay_store_persists_pending_envelopes_across_router_restarts() {
        let database_path =
            std::env::temp_dir().join(format!("messenger-relay-{}.sqlite", Uuid::new_v4()));
        let database_path = database_path.to_string_lossy().into_owned();
        let alice = IdentityKeypair::generate();
        let bob = IdentityKeypair::generate();
        let plaintext = b"persistent relay message";
        let envelope = alice
            .encrypt_for(&bob.public_identity(), plaintext)
            .unwrap();

        let first_app = sqlite_app(&database_path);
        let alice_session = authenticate_peer(first_app.clone(), &alice).await;
        let submit: SubmitEnvelopeResponse = request_json(
            first_app,
            Request::builder()
                .method(Method::POST)
                .uri("/v1/relay/envelopes")
                .header(header::AUTHORIZATION, bearer(&alice_session)),
            &SubmitEnvelopeRequest { envelope },
        )
        .await;
        assert!(submit.accepted);

        let restarted_app = sqlite_app(&database_path);
        let bob_session = authenticate_peer(restarted_app.clone(), &bob).await;
        let pending: PendingEnvelopesResponse = request_empty(
            restarted_app.clone(),
            Request::builder()
                .method(Method::GET)
                .uri("/v1/relay/envelopes/pending")
                .header(header::AUTHORIZATION, bearer(&bob_session)),
        )
        .await;

        assert_eq!(pending.envelopes.len(), 1);
        let decrypted = bob
            .decrypt_from(&alice.public_identity(), &pending.envelopes[0])
            .unwrap();
        assert_eq!(decrypted, plaintext);

        let delivered: MarkDeliveredResponse = request_empty(
            restarted_app,
            Request::builder()
                .method(Method::POST)
                .uri(format!(
                    "/v1/relay/envelopes/{}/delivered",
                    pending.envelopes[0].message_id.as_uuid()
                ))
                .header(header::AUTHORIZATION, bearer(&bob_session)),
        )
        .await;
        assert!(delivered.removed);

        let _ = std::fs::remove_file(database_path);
    }

    fn sqlite_app(database_path: &str) -> Router {
        let store = SqliteRelayStore::connect(database_path).unwrap();
        build_router(AppState::with_relay_store(Arc::new(store)))
    }

    async fn authenticate_peer(app: Router, identity: &IdentityKeypair) -> String {
        let challenge: AuthChallenge = request_json(
            app.clone(),
            Request::builder()
                .method(Method::POST)
                .uri("/v1/auth/challenge"),
            &AuthChallengeRequest {
                peer_id: identity.peer_id(),
            },
        )
        .await;
        let response: AuthVerifyResponse = request_json(
            app,
            Request::builder()
                .method(Method::POST)
                .uri("/v1/auth/verify"),
            &AuthVerifyRequest {
                identity: public_identity_document(identity),
                challenge_id: challenge.challenge_id.clone(),
                signature: identity.sign_auth_challenge(&challenge),
            },
        )
        .await;

        assert_eq!(response.peer_id, identity.peer_id());
        response.session_token
    }

    fn public_identity_document(identity: &IdentityKeypair) -> PublicIdentityDocument {
        let public = identity.public_identity();
        PublicIdentityDocument {
            peer_id: public.peer_id,
            signing_key: public.signing_key,
            agreement_key: public.agreement_key,
        }
    }

    async fn request_json<TRequest, TResponse>(
        app: Router,
        builder: Builder,
        payload: &TRequest,
    ) -> TResponse
    where
        TRequest: Serialize,
        TResponse: DeserializeOwned,
    {
        let request = builder
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(payload).unwrap()))
            .unwrap();
        send(app, request).await
    }

    async fn request_empty<TResponse>(app: Router, builder: Builder) -> TResponse
    where
        TResponse: DeserializeOwned,
    {
        let request = builder.body(Body::empty()).unwrap();
        send(app, request).await
    }

    async fn send<TResponse>(app: Router, request: Request<Body>) -> TResponse
    where
        TResponse: DeserializeOwned,
    {
        let response = app.oneshot(request).await.unwrap();
        assert!(
            response.status().is_success(),
            "unexpected response status: {}",
            response.status()
        );
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    fn bearer(token: &str) -> String {
        format!("Bearer {token}")
    }
}
