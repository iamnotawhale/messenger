# Flutter client

This directory contains the first cross-platform Flutter shell targeting:

- desktop: Linux, macOS, Windows
- mobile: Android, iOS

The current cloud image does not include the Flutter SDK, so platform directories
and generated `flutter_rust_bridge` bindings are intentionally not checked in yet.
The Dart sources model the app shell and bridge contract that will be wired to
generated bindings once Flutter is available.

Planned client layers:

```text
lib/
  main.dart
  src/
    bridge/
      messenger_bridge.dart
      messenger_bridge_factory.dart
      generated_messenger_api.dart
      mock_messenger_bridge.dart
      rust_messenger_bridge.dart
      generated/
    models/
    screens/
    state/
```

`MessengerBridge` is the app-facing interface. The app uses
`createMessengerBridge` from `messenger_bridge_factory.dart`:

- default/debug mode uses `MockMessengerBridge`, so the shell can run as a demo;
- generated mode uses `RustMessengerBridge` once `flutter_rust_bridge` bindings
  are available and `MESSENGER_USE_RUST_BRIDGE=true` is set.

After running bridge generation, launch with generated Rust bindings enabled:

```bash
flutter run --dart-define=MESSENGER_USE_RUST_BRIDGE=true
```
