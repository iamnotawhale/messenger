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
  flutter/                Flutter app shell and bridge-facing UI
crates/
  messenger-client/       Client service layer
  messenger-client-store/ SQLite-backed local client store
  messenger-core/         Application orchestration layer
  messenger-crypto/       Identity keys, encryption, signatures
  messenger-ffi/          Flutter bridge facade DTOs and functions
  messenger-protocol/     Wire/domain protocol types
  messenger-storage/      Storage interfaces
  messenger-transport/    Relay HTTP client
docs/
  architecture.md         Current architecture notes
  flutter-bridge.md       Flutter bridge facade and app workflow
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
messenger-ffi facade
  |-- protocol envelopes
  |-- identity and crypto
  |-- local storage
  |-- transport abstraction
        |-- relay
        |-- WebRTC P2P
```

## Development

```bash
cargo fmt --all --check
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p messenger-server
```

GitHub Actions runs the Rust checks, the relay smoke test, and a Flutter job
that generates `flutter_rust_bridge` bindings before running `flutter analyze`
and `flutter test`.

By default the relay queue is in memory. Set `MESSENGER_SQLITE_PATH` to persist
encrypted relay envelopes across restarts:

```bash
MESSENGER_SQLITE_PATH=./relay.db cargo run -p messenger-server
```

Set `MESSENGER_BIND_ADDR` to change the listen address:

```bash
MESSENGER_BIND_ADDR=127.0.0.1:18080 cargo run -p messenger-server
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

Auth is a signed challenge flow. The server stores session tokens in memory and
relay queues in memory by default for local development. Set
`MESSENGER_SQLITE_PATH`, for example `MESSENGER_SQLITE_PATH=./relay.db`, to
persist relay envelopes across server restarts.

## Dev CLI

The `messenger-dev` tool exercises the relay flow without the Flutter client:

```bash
cargo run -p messenger-dev -- identity new alice.json
cargo run -p messenger-dev -- identity new bob.json
cargo run -p messenger-dev -- identity public alice.json alice.public.json
cargo run -p messenger-dev -- identity public bob.json bob.public.json

cargo run -p messenger-dev -- send \
  --server http://127.0.0.1:8080 \
  --from alice.json \
  --to bob.public.json \
  --text "hello"

cargo run -p messenger-dev -- receive \
  --server http://127.0.0.1:8080 \
  --identity bob.json \
  --from alice.public.json
```

The CLI also has a database-backed workflow that is closer to the future app:

```bash
cargo run -p messenger-dev -- db init --db alice.db
cargo run -p messenger-dev -- db init --db bob.db
cargo run -p messenger-dev -- db public --db alice.db --out alice.public.json
cargo run -p messenger-dev -- db public --db bob.db --out bob.public.json
cargo run -p messenger-dev -- contact add --db alice.db --name Bob --public bob.public.json
cargo run -p messenger-dev -- contact add --db bob.db --name Alice --public alice.public.json
cargo run -p messenger-dev -- message send --db alice.db --to Bob --text "hello"
cargo run -p messenger-dev -- sync --db bob.db
cargo run -p messenger-dev -- messages list --db bob.db --contact Alice
```

For a full local smoke test that starts the server, creates temporary Alice/Bob
identities, sends an encrypted message, receives it, and verifies that Bob's
queue is empty after delivery:

```bash
scripts/dev-relay-smoke.sh
```

For the database-backed client workflow, run:

```bash
scripts/dev-db-smoke.sh
```

Generate Flutter/Rust bridge bindings in an environment with Flutter, Dart, and
`flutter_rust_bridge_codegen` installed:

```bash
scripts/generate-flutter-bridge.sh
```

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
