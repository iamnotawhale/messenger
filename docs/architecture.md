# Architecture

## Goal

Build a small but serious P2P-first messenger for desktop, Android, and iOS. The system prefers direct encrypted peer-to-peer delivery, while an encrypted relay keeps the application useful when peers are offline or unreachable.

## Client

```text
Flutter UI
   |
flutter_rust_bridge
   |
Rust core
   |-- identity
   |-- crypto
   |-- protocol
   |-- storage
   |-- sync
   |-- transport abstraction
          |-- relay
          |-- WebRTC data channel
```

Flutter owns presentation, platform permissions, notifications, and OS secure-storage bindings. Rust owns invariants and any behavior that should be identical on all platforms.

## Server

```text
Rust server
   |-- health API
   |-- relay API
   |-- WebRTC signaling
   |-- presence
   |-- push wakeups
   |-- message retention jobs
```

The server is not trusted with plaintext. It stores encrypted envelopes and minimal routing metadata.

## Identity

- Long-term identity is based on an Ed25519 signing key.
- `PeerId` is a BLAKE3 hash-derived identifier over public identity material.
- Contacts are verified out of band with QR codes or fingerprints.
- Key changes are explicit trust events, not silent updates.

## Message delivery

1. Client creates a typed `PlainMessage`.
2. Rust core encrypts it for the recipient and signs the envelope.
3. Transport layer attempts direct WebRTC delivery when the recipient is online.
4. Relay is used when direct delivery is unavailable or the recipient is offline.
5. Receiver verifies the signature, decrypts the payload, deduplicates by message id, and persists the result locally.

## Storage

Initial local storage is represented by repository traits so the core can be tested without SQLite. A later implementation should use SQLite with encrypted fields or SQLCipher and protect root keys with:

- iOS Keychain
- Android Keystore
- macOS Keychain
- Windows Credential Manager
- Linux Secret Service

## Future protocol upgrades

The first protocol is intentionally a signed sealed-message model. The type boundaries leave room for:

- Double Ratchet for one-to-one conversations
- MLS for groups
- multiple devices per identity
- multiple relay servers
- optional LAN discovery
