import '../models/chat_message.dart';
import '../models/contact.dart';
import 'messenger_bridge.dart';

class MockMessengerBridge implements MessengerBridge {
  MockMessengerBridge({
    this.peerId = 'peer:local-demo',
    List<Contact>? initialContacts,
  }) : _contacts = List<Contact>.of(initialContacts ?? const []);

  final String peerId;
  final List<Contact> _contacts;
  final Map<String, List<ChatMessage>> _messagesByContact = {};
  var _nextMessageId = 1;

  @override
  Future<void> addContact(
    ClientConfig config, {
    required String name,
    required String publicIdentityJson,
  }) async {
    final peerId = _extractPeerId(publicIdentityJson) ?? 'peer:mock-${_contacts.length + 1}';
    final existingIndex = _contacts.indexWhere((contact) => contact.name == name);
    final contact = Contact(name: name, peerId: peerId);
    if (existingIndex >= 0) {
      _contacts[existingIndex] = contact;
    } else {
      _contacts.add(contact);
    }
  }

  @override
  Future<String> exportPublicIdentity(ClientConfig config) async {
    return '{"peer_id":"$peerId","signing_key":[],"agreement_key":[]}';
  }

  @override
  Future<String> initClient(ClientConfig config) async {
    return peerId;
  }

  @override
  Future<List<Contact>> listContacts(ClientConfig config) async {
    return List<Contact>.unmodifiable(_contacts);
  }

  @override
  Future<List<ChatMessage>> listMessages(
    ClientConfig config, {
    required String contactName,
  }) async {
    return List<ChatMessage>.unmodifiable(_messagesByContact[contactName] ?? const []);
  }

  @override
  Future<String> sendMessage(
    ClientConfig config, {
    required String contactName,
    required String body,
  }) async {
    final id = 'mock:${_nextMessageId++}';
    final messages = _messagesByContact.putIfAbsent(contactName, () => []);
    messages.add(ChatMessage(
      messageId: id,
      contactName: contactName,
      direction: MessageDirection.outbound,
      body: body,
      createdAtMs: DateTime.now().millisecondsSinceEpoch,
    ));
    return id;
  }

  @override
  Future<List<ChatMessage>> sync(ClientConfig config) async {
    return const [];
  }

  String? _extractPeerId(String publicIdentityJson) {
    final match = RegExp(r'"peer_id"\s*:\s*"([^"]+)"').firstMatch(publicIdentityJson);
    return match?.group(1);
  }
}
