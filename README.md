# Messenger

P2P-first encrypted messenger for desktop, Android, and iOS.

The product direction is:

- Flutter client for cross-platform UI.
- Rust core for identity, cryptography, protocol, storage, and sync.
- WebRTC data channels for direct peer-to-peer delivery.
- Encrypted relay fallback for offline or unreachable devices.
- Rust backend for signaling, relay queues, presence, and push wakeups.

This repository starts with the shared Rust foundation. The Flutter app folder is
kept as an integration target until the Flutter SDK is available in the
development environment.

## Repository layout

```text
apps/
  flutter/                Flutter client integration notes and future app
crates/
  messenger-core/         Application orchestration layer
  messenger-crypto/       Identity keys, encryption, signatures
  messenger-protocol/     Wire/domain protocol types
  messenger-storage/      Storage interfaces
docs/
  architecture.md         Current architecture notes
server/
  messenger-server/       Relay/signaling server skeleton
```

## Initial architecture

The first usable version should deliver messages through the encrypted relay
path. WebRTC P2P is added behind the same transport boundary once the core
message model is stable.

```text
Flutter UI
  |
flutter_rust_bridge
  |
Rust core
  |-- protocol envelopes
  |-- identity and crypto
  |-- local storage
  |-- transport abstraction
        |-- relay
        |-- WebRTC P2P
```

## Development

```bash
cargo test --workspace
cargo run -p messenger-server
```

By default the relay queue is in memory. Set `MESSENGER_SQLITE_PATH` to persist
encrypted relay envelopes across restarts:

```bash
MESSENGER_SQLITE_PATH=./relay.db cargo run -p messenger-server
```

The server currently exposes a health endpoint:

```bash
curl http://127.0.0.1:8080/health
```

Relay API skeleton:

```text
POST /v1/auth/challenge
POST /v1/auth/verify
POST /v1/relay/envelopes
GET  /v1/relay/envelopes/pending
POST /v1/relay/envelopes/{message_id}/delivered
```

Auth is a signed challenge flow. The server stores session tokens and relay
queues in memory by default for local development. Set `MESSENGER_SQLITE_URL`
to a SQLite connection string, for example `sqlite://messenger.db?mode=rwc`, to
persist relay envelopes across server restarts.

## Security direction

The current crypto crate provides an MVP sealed-message primitive:

- Ed25519 identity signing keys.
- X25519 key agreement.
- BLAKE3 key derivation.
- XChaCha20-Poly1305 authenticated encryption.
- Signed encrypted envelopes.

This is intended as a foundation, not the final advanced chat protocol. The
future session layer should add a ratcheting protocol for 1:1 chats and MLS or
another reviewed group protocol for groups.
