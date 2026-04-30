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
    models/
    screens/
    state/
```

`MessengerBridge` is currently an interface. A later Flutter setup pass should
generate concrete bindings from `crates/messenger-ffi` and implement that
interface with Rust calls.
