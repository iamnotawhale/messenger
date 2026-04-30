# Flutter client

This directory is reserved for the cross-platform Flutter application targeting:

- desktop: Linux, macOS, Windows
- mobile: Android, iOS

The current cloud image does not include the Flutter SDK, so the initial commit keeps the
client as a documented integration point instead of checking in generated Flutter files.

Planned client layers:

```text
lib/
  app/
    navigation
    theme
  features/
    onboarding
    contacts
    conversations
    settings
  bridge/
    generated flutter_rust_bridge bindings
```

The client should call into `messenger-core` through `flutter_rust_bridge` for all protocol,
crypto, storage, and sync behavior. Dart code should stay focused on UI state and platform
integration.
