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
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use uuid::Uuid;

const AUTH_HEADER: &str = "authorization";
const CHALLENGE_TTL: Duration = Duration::from_secs(5 * 60);

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    protocol_version: u16,
    transports: [TransportKind; 2],
}

#[derive(Clone, Default)]
struct AppState {
    challenges: Arc<Mutex<HashMap<String, PendingChallenge>>>,
    sessions: Arc<Mutex<HashMap<String, PublicIdentity>>>,
    queues: Arc<Mutex<HashMap<String, Vec<Envelope>>>>,
}

#[derive(Debug, Clone)]
struct PendingChallenge {
    challenge: AuthChallenge,
}

#[derive(Debug, Serialize, Deserialize)]
struct ErrorResponse {
    error: &'static str,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = AppState::default();
    let app = Router::new()
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
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080")
        .await
        .unwrap_or_else(|error| panic!("failed to bind server: {error}"));

    axum::serve(listener, app)
        .await
        .unwrap_or_else(|error| panic!("server failed: {error}"));
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
    let recipient = request.envelope.recipient.as_str().to_owned();
    state
        .queues
        .lock()
        .map_err(|_| internal_error())?
        .entry(recipient)
        .or_default()
        .push(request.envelope);

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
        .queues
        .lock()
        .map_err(|_| internal_error())?
        .get(identity.peer_id.as_str())
        .cloned()
        .unwrap_or_default();

    Ok(Json(PendingEnvelopesResponse { envelopes }))
}

async fn mark_delivered(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(message_id): Path<Uuid>,
) -> Result<Json<MarkDeliveredResponse>, (StatusCode, Json<ErrorResponse>)> {
    let identity = authenticate(&state, &headers)?;
    let mut queues = state.queues.lock().map_err(|_| internal_error())?;
    let queue = queues
        .entry(identity.peer_id.as_str().to_owned())
        .or_default();
    let before = queue.len();
    queue.retain(|envelope| envelope.message_id.as_uuid() != message_id);

    Ok(Json(MarkDeliveredResponse {
        removed: before != queue.len(),
    }))
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
