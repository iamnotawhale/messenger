import 'generated_messenger_api.dart';
import 'messenger_bridge.dart';
import 'mock_messenger_bridge.dart';
import 'rust_messenger_bridge.dart';

const _useMockBridge = bool.fromEnvironment('MESSENGER_USE_MOCK_BRIDGE');
const _useRustBridge = bool.fromEnvironment('MESSENGER_USE_RUST_BRIDGE');

Future<MessengerBridge> createMessengerBridge() async {
  return createDefaultMessengerBridge(
    useMock: _useMockBridge,
    useRustBridge: _useRustBridge,
  );
}

Future<MessengerBridge> createDefaultMessengerBridge({
  bool useMock = false,
  bool useRustBridge = false,
}) async {
  if (useMock || !useRustBridge) {
    return MockMessengerBridge();
  }

  return RustMessengerBridge(createGeneratedMessengerApi());
}
