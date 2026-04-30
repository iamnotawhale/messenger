import 'package:flutter_test/flutter_test.dart';
import 'package:messenger_app/src/bridge/messenger_bridge.dart';
import 'package:messenger_app/src/bridge/mock_messenger_bridge.dart';
import 'package:messenger_app/src/state/messenger_controller.dart';

void main() {
  test('controller initializes and sends messages', () async {
    final bridge = MockMessengerBridge();
    final controller = MessengerController(
      bridge: bridge,
      config: const ClientConfig(
        databasePath: 'client.db',
        relayUrl: 'http://127.0.0.1:8080',
      ),
    );

    await controller.initialize();
    await controller.addContact(name: 'Bob', publicIdentityJson: '{"peer_id":"peer:bob"}');
    await controller.selectContact('Bob');
    await controller.sendMessage('hello');

    expect(controller.peerId, 'peer:local-demo');
    expect(controller.contacts.single.name, 'Bob');
    expect(controller.messages.single.body, 'hello');
  });
}
