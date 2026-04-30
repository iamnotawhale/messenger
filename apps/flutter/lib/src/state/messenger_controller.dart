import 'package:flutter/foundation.dart';

import '../bridge/messenger_bridge.dart';
import '../models/chat_message.dart';
import '../models/contact.dart';

class MessengerController extends ChangeNotifier {
  MessengerController({
    required MessengerBridge bridge,
    required ClientConfig config,
  })  : _bridge = bridge,
        _config = config;

  final MessengerBridge _bridge;
  final ClientConfig _config;

  String? _peerId;
  String? _selectedContact;
  List<Contact> _contacts = const [];
  List<ChatMessage> _messages = const [];
  bool _busy = false;
  String? _error;

  String? get peerId => _peerId;
  String? get selectedContact => _selectedContact;
  List<Contact> get contacts => _contacts;
  List<ChatMessage> get messages => _messages;
  bool get busy => _busy;
  String? get error => _error;

  Future<void> initialize() async {
    await _run(() async {
      _peerId = await _bridge.initClient(_config);
      _contacts = await _bridge.listContacts(_config);
      if (_contacts.isNotEmpty) {
        await selectContact(_contacts.first.name);
      }
    });
  }

  Future<String> exportPublicIdentity() {
    return _bridge.exportPublicIdentity(_config);
  }

  Future<void> addContact({
    required String name,
    required String publicIdentityJson,
  }) async {
    await _run(() async {
      await _bridge.addContact(
        _config,
        name: name,
        publicIdentityJson: publicIdentityJson,
      );
      _contacts = await _bridge.listContacts(_config);
    });
  }

  Future<void> selectContact(String name) async {
    _selectedContact = name;
    _messages = await _bridge.listMessages(_config, contactName: name);
    notifyListeners();
  }

  Future<void> sendMessage(String body) async {
    final contact = _selectedContact;
    if (contact == null || body.trim().isEmpty) {
      return;
    }

    await _run(() async {
      await _bridge.sendMessage(_config, contactName: contact, body: body);
      _messages = await _bridge.listMessages(_config, contactName: contact);
    });
  }

  Future<void> sync() async {
    await _run(() async {
      await _bridge.sync(_config);
      _contacts = await _bridge.listContacts(_config);
      final contact = _selectedContact;
      if (contact != null) {
        _messages = await _bridge.listMessages(_config, contactName: contact);
      }
    });
  }

  Future<void> _run(Future<void> Function() action) async {
    _busy = true;
    _error = null;
    notifyListeners();
    try {
      await action();
    } catch (error) {
      _error = error.toString();
    } finally {
      _busy = false;
      notifyListeners();
    }
  }
}
