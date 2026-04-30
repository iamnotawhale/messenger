import 'package:flutter_test/flutter_test.dart';
import 'package:messenger_app/src/bridge/messenger_bridge_factory.dart';
import 'package:messenger_app/src/bridge/mock_messenger_bridge.dart';

void main() {
  test('uses mock bridge when requested', () async {
    final bridge = await createDefaultMessengerBridge(useMock: true);

    expect(bridge, isA<MockMessengerBridge>());
  });
}
