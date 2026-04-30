# Flutter bridge

The Rust-to-Flutter boundary is intentionally kept behind `messenger-ffi`.
Flutter should depend on simple DTOs and commands rather than domain internals
such as `PeerId`, `Envelope`, SQLite rows, or transport sessions.

## Rust facade

`crates/messenger-ffi` exposes the app-shaped API:

```text
init_client(config) -> peer_id
export_public_identity(config) -> public_identity_json
add_contact(config, name, public_identity_json)
list_contacts(config) -> ContactDto[]
send_message(config, contact_name, body) -> message_id
sync(config) -> SyncedMessageDto[]
list_messages(config, contact_name) -> MessageDto[]
```

The facade opens the local SQLite client database, uses the configured relay
URL, and delegates to `messenger-client`.

## Flutter shell

`apps/flutter` contains a minimal app shell:

- `MessengerBridge` defines the Dart-side bridge contract.
- `MockMessengerBridge` lets the UI run before generated bindings exist.
- `MessengerController` owns UI state and calls bridge methods.
- `HomeScreen` provides onboarding, public identity export, contact add, send,
  sync, and message list flows.

## Next integration step

When the Flutter SDK and `flutter_rust_bridge` are available:

1. Generate Dart bindings for `messenger-ffi`.
2. Implement `MessengerBridge` with generated Rust calls.
3. Add platform-specific secure storage for DB path and app lock settings.
4. Replace the mock bridge in `main.dart` with the generated bridge.
