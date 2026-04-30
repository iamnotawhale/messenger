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
      mock_messenger_bridge.dart
      rust_messenger_bridge.dart
      generated/
    models/
    screens/
    state/
```

`MessengerBridge` is the app-facing interface. `MockMessengerBridge` is the
default app bridge so the shell can run as a demo before native bindings are
available. `RustMessengerBridge` is prepared to call generated
`flutter_rust_bridge` bindings once `flutter_rust_bridge.yaml` is generated in
an environment with Flutter/Dart installed.
