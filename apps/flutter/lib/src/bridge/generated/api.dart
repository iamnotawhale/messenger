// Placeholder for flutter_rust_bridge generated bindings.
//
// This file keeps the Flutter shell analyzable before code generation runs.
// `scripts/generate-flutter-bridge.sh` is expected to overwrite it with the
// real generated API in environments with Flutter/Dart tooling installed.

class ClientConfig {
  const ClientConfig({
    required this.databasePath,
    required this.relayUrl,
  });

  final String databasePath;
  final String relayUrl;
}

class ContactDto {
  const ContactDto({
    required this.name,
    required this.peerId,
  });

  final String name;
  final String peerId;
}

class MessageDto {
  const MessageDto({
    required this.messageId,
    required this.contactName,
    required this.direction,
    required this.body,
    required this.createdAtMs,
  });

  final String messageId;
  final String contactName;
  final String direction;
  final String body;
  final int createdAtMs;
}

class SyncedMessageDto {
  const SyncedMessageDto({
    required this.messageId,
    required this.senderPeerId,
    required this.body,
  });

  final String messageId;
  final String senderPeerId;
  final String body;
}

Never _missingGeneratedBindings() {
  throw UnimplementedError(
    'flutter_rust_bridge bindings have not been generated yet. '
    'Run scripts/generate-flutter-bridge.sh.',
  );
}

Future<String> initClient({required ClientConfig config}) {
  return _missingGeneratedBindings();
}

Future<String> exportPublicIdentity({required ClientConfig config}) {
  return _missingGeneratedBindings();
}

Future<void> addContact({
  required ClientConfig config,
  required String name,
  required String publicIdentityJson,
}) {
  return _missingGeneratedBindings();
}

Future<List<ContactDto>> listContacts({required ClientConfig config}) {
  return _missingGeneratedBindings();
}

Future<String> sendMessage({
  required ClientConfig config,
  required String contactName,
  required String body,
}) {
  return _missingGeneratedBindings();
}

Future<List<SyncedMessageDto>> sync({required ClientConfig config}) {
  return _missingGeneratedBindings();
}

Future<List<MessageDto>> listMessages({
  required ClientConfig config,
  required String contactName,
}) {
  return _missingGeneratedBindings();
}
