import '../models/chat_message.dart';
import '../models/contact.dart';

class ClientConfig {
  const ClientConfig({
    required this.databasePath,
    required this.relayUrl,
  });

  final String databasePath;
  final String relayUrl;
}

abstract interface class MessengerBridge {
  Future<String> initClient(ClientConfig config);

  Future<String> exportPublicIdentity(ClientConfig config);

  Future<void> addContact(
    ClientConfig config, {
    required String name,
    required String publicIdentityJson,
  });

  Future<List<Contact>> listContacts(ClientConfig config);

  Future<String> sendMessage(
    ClientConfig config, {
    required String contactName,
    required String body,
  });

  Future<List<ChatMessage>> sync(ClientConfig config);

  Future<List<ChatMessage>> listMessages(
    ClientConfig config, {
    required String contactName,
  });
}

class UnimplementedMessengerBridge implements MessengerBridge {
  const UnimplementedMessengerBridge();

  @override
  Future<void> addContact(
    ClientConfig config, {
    required String name,
    required String publicIdentityJson,
  }) async {
    throw UnimplementedError('flutter_rust_bridge bindings are not generated yet');
  }

  @override
  Future<String> exportPublicIdentity(ClientConfig config) async {
    throw UnimplementedError('flutter_rust_bridge bindings are not generated yet');
  }

  @override
  Future<String> initClient(ClientConfig config) async {
    throw UnimplementedError('flutter_rust_bridge bindings are not generated yet');
  }

  @override
  Future<List<Contact>> listContacts(ClientConfig config) async {
    throw UnimplementedError('flutter_rust_bridge bindings are not generated yet');
  }

  @override
  Future<List<ChatMessage>> listMessages(
    ClientConfig config, {
    required String contactName,
  }) async {
    throw UnimplementedError('flutter_rust_bridge bindings are not generated yet');
  }

  @override
  Future<String> sendMessage(
    ClientConfig config, {
    required String contactName,
    required String body,
  }) async {
    throw UnimplementedError('flutter_rust_bridge bindings are not generated yet');
  }

  @override
  Future<List<ChatMessage>> sync(ClientConfig config) async {
    throw UnimplementedError('flutter_rust_bridge bindings are not generated yet');
  }
}
