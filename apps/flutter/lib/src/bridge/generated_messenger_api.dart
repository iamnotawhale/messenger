import '../models/chat_message.dart';
import '../models/contact.dart';
import 'generated/api.dart' as generated;

Future<GeneratedMessengerApi?> createGeneratedMessengerApi() async {
  return const GeneratedMessengerApi();
}

class GeneratedMessengerApi {
  const GeneratedMessengerApi();

  generated.ClientConfig createClientConfig({
    required String databasePath,
    required String relayUrl,
  }) {
    return generated.ClientConfig(
      databasePath: databasePath,
      relayUrl: relayUrl,
    );
  }

  Future<String> initClient({required generated.ClientConfig config}) {
    return generated.initClient(config: config);
  }

  Future<String> exportPublicIdentity({required generated.ClientConfig config}) {
    return generated.exportPublicIdentity(config: config);
  }

  Future<void> addContact({
    required generated.ClientConfig config,
    required String name,
    required String publicIdentityJson,
  }) {
    return generated.addContact(
      config: config,
      name: name,
      publicIdentityJson: publicIdentityJson,
    );
  }

  Future<List<Contact>> listContacts({required generated.ClientConfig config}) async {
    final contacts = await generated.listContacts(config: config);
    return contacts
        .map((contact) => Contact(
              name: contact.name,
              peerId: contact.peerId,
            ))
        .toList(growable: false);
  }

  Future<String> sendMessage({
    required generated.ClientConfig config,
    required String contactName,
    required String body,
  }) {
    return generated.sendMessage(
      config: config,
      contactName: contactName,
      body: body,
    );
  }

  Future<List<ChatMessage>> sync({required generated.ClientConfig config}) async {
    final messages = await generated.sync(config: config);
    return messages
        .map((message) => ChatMessage(
              messageId: message.messageId,
              contactName: message.senderPeerId,
              direction: MessageDirection.inbound,
              body: message.body,
              createdAtMs: 0,
            ))
        .toList(growable: false);
  }

  Future<List<ChatMessage>> listMessages({
    required generated.ClientConfig config,
    required String contactName,
  }) async {
    final messages = await generated.listMessages(
      config: config,
      contactName: contactName,
    );
    return messages
        .map((message) => ChatMessage(
              messageId: message.messageId,
              contactName: message.contactName,
              direction: message.direction == 'outbound'
                  ? MessageDirection.outbound
                  : MessageDirection.inbound,
              body: message.body,
              createdAtMs: message.createdAtMs,
            ))
        .toList(growable: false);
  }
}
