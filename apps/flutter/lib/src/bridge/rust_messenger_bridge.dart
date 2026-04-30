import '../models/chat_message.dart';
import '../models/contact.dart';
import 'messenger_bridge.dart';

// This adapter is intentionally written against a tiny dynamic boundary so the
// app shell can compile before flutter_rust_bridge generated files are checked
// in. Once bindings are generated, pass the generated API object here.
class RustMessengerBridge implements MessengerBridge {
  const RustMessengerBridge(this._api);

  final dynamic _api;

  @override
  Future<void> addContact(
    ClientConfig config, {
    required String name,
    required String publicIdentityJson,
  }) async {
    await _api.addContact(
      config: _config(config),
      name: name,
      publicIdentityJson: publicIdentityJson,
    );
  }

  @override
  Future<String> exportPublicIdentity(ClientConfig config) {
    return _api.exportPublicIdentity(config: _config(config));
  }

  @override
  Future<String> initClient(ClientConfig config) {
    return _api.initClient(config: _config(config));
  }

  @override
  Future<List<Contact>> listContacts(ClientConfig config) async {
    final contacts = await _api.listContacts(config: _config(config)) as List;
    return contacts
        .map((contact) => Contact(
              name: contact.name as String,
              peerId: contact.peerId as String,
            ))
        .toList(growable: false);
  }

  @override
  Future<List<ChatMessage>> listMessages(
    ClientConfig config, {
    required String contactName,
  }) async {
    final messages = await _api.listMessages(
      config: _config(config),
      contactName: contactName,
    ) as List;
    return messages.map(_messageFromGenerated).toList(growable: false);
  }

  @override
  Future<String> sendMessage(
    ClientConfig config, {
    required String contactName,
    required String body,
  }) {
    return _api.sendMessage(
      config: _config(config),
      contactName: contactName,
      body: body,
    );
  }

  @override
  Future<List<ChatMessage>> sync(ClientConfig config) async {
    final messages = await _api.sync(config: _config(config)) as List;
    return messages
        .map((message) => ChatMessage(
              messageId: message.messageId as String,
              contactName: message.senderPeerId as String,
              direction: MessageDirection.inbound,
              body: message.body as String,
              createdAtMs: 0,
            ))
        .toList(growable: false);
  }

  dynamic _config(ClientConfig config) {
    return _api.createClientConfig(
      databasePath: config.databasePath,
      relayUrl: config.relayUrl,
    );
  }

  ChatMessage _messageFromGenerated(dynamic message) {
    return ChatMessage(
      messageId: message.messageId as String,
      contactName: message.contactName as String,
      direction: message.direction == 'outbound'
          ? MessageDirection.outbound
          : MessageDirection.inbound,
      body: message.body as String,
      createdAtMs: message.createdAtMs as int,
    );
  }
}
