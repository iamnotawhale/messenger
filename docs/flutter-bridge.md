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

## flutter_rust_bridge scaffold

The repository now includes `flutter_rust_bridge.yaml`, pointing codegen at:

- Rust input: `crates/messenger-ffi/src/api.rs`
- Dart output: `apps/flutter/lib/src/bridge/generated/api.dart`
- C output: `apps/flutter/rust/generated/frb_generated.h`

`apps/flutter/lib/src/bridge/rust_messenger_bridge.dart` is the adapter that
will wrap generated functions and satisfy the app's `MessengerBridge` interface.

Generated binding files are intentionally not checked in yet because this cloud
image does not include Flutter/Dart tooling.

## Next integration step

When the Flutter SDK and `flutter_rust_bridge` are available:

1. Install the generator: `cargo install flutter_rust_bridge_codegen`.
2. From the repository root, run `scripts/generate-flutter-bridge.sh`.
3. Run `flutter pub get`, `flutter analyze`, and `flutter test` in
   `apps/flutter`.
4. Replace the mock bridge in `main.dart` with `RustMessengerBridge`.

The generation script checks for `flutter`, `dart`, and
`flutter_rust_bridge_codegen` before running codegen.
