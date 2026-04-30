import 'package:flutter_test/flutter_test.dart';
import 'package:messenger_app/src/bridge/messenger_bridge.dart';
import 'package:messenger_app/src/models/contact.dart';
import 'package:messenger_app/src/models/chat_message.dart';
import 'package:messenger_app/src/state/messenger_controller.dart';

class FakeBridge implements MessengerBridge {
  var initialized = false;
  final contacts = <Contact>[];
  final messages = <ChatMessage>[];

  @override
  Future<String> initClient(ClientConfig config) async {
    initialized = true;
    return 'peer:test';
  }

  @override
  Future<String> exportPublicIdentity(ClientConfig config) async {
    return '{"peer_id":"peer:test"}';
  }

  @override
  Future<void> addContact(
    ClientConfig config,
    String name,
    String publicIdentityJson,
  ) async {
    contacts.add(Contact(name: name, peerId: 'peer:bob'));
  }

  @override
  Future<List<Contact>> listContacts(ClientConfig config) async => contacts;

  @override
  Future<String> sendMessage(
    ClientConfig config,
    String contactName,
    String body,
  ) async {
    messages.add(ChatMessage(
      messageId: 'message:1',
      contactName: contactName,
      direction: MessageDirection.outbound,
      body: body,
      createdAtMs: 1,
    ));
    return 'message:1';
  }

  @override
  Future<List<SyncedMessage>> sync(ClientConfig config) async => const [];

  @override
  Future<List<ChatMessage>> listMessages(
    ClientConfig config,
    String contactName,
  ) async =>
      messages;
}

void main() {
  test('controller initializes and sends messages', () async {
    final bridge = FakeBridge();
    final controller = MessengerController(
      bridge: bridge,
      config: const ClientConfig(
        databasePath: 'client.db',
        relayUrl: 'http://127.0.0.1:8080',
      ),
    );

    await controller.initialize();
    await controller.addContact('Bob', '{}');
    await controller.sendMessage('Bob', 'hello');

    expect(controller.peerId, 'peer:test');
    expect(controller.contacts.single.name, 'Bob');
    expect(controller.messages.single.body, 'hello');
  });
}
