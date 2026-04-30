import 'package:flutter/material.dart';

import '../state/messenger_controller.dart';

class HomeScreen extends StatefulWidget {
  const HomeScreen({super.key, required this.controller});

  final MessengerController controller;

  @override
  State<HomeScreen> createState() => _HomeScreenState();
}

class _HomeScreenState extends State<HomeScreen> {
  final TextEditingController _contactNameController = TextEditingController();
  final TextEditingController _publicIdentityController = TextEditingController();
  final TextEditingController _messageController = TextEditingController();
  String? _selectedContact;

  @override
  void initState() {
    super.initState();
    widget.controller.addListener(_onControllerChanged);
    widget.controller.initialize();
  }

  @override
  void dispose() {
    widget.controller.removeListener(_onControllerChanged);
    _contactNameController.dispose();
    _publicIdentityController.dispose();
    _messageController.dispose();
    super.dispose();
  }

  void _onControllerChanged() {
    if (!mounted) {
      return;
    }
    setState(() {
      if (_selectedContact == null && widget.controller.contacts.isNotEmpty) {
        _selectedContact = widget.controller.contacts.first.name;
      }
    });
  }

  @override
  Widget build(BuildContext context) {
    final controller = widget.controller;
    final contacts = controller.contacts;
    final selectedContact = _selectedContact;
    final messages = selectedContact == null ? const [] : controller.messagesFor(selectedContact);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Messenger'),
        actions: [
          IconButton(
            onPressed: controller.sync,
            icon: const Icon(Icons.sync),
            tooltip: 'Sync',
          ),
        ],
      ),
      body: Row(
        children: [
          SizedBox(
            width: 320,
            child: ListView(
              padding: const EdgeInsets.all(16),
              children: [
                Text('Peer ID', style: Theme.of(context).textTheme.titleMedium),
                SelectableText(controller.peerId ?? 'Not initialized'),
                const SizedBox(height: 24),
                Text('Add contact', style: Theme.of(context).textTheme.titleMedium),
                TextField(
                  controller: _contactNameController,
                  decoration: const InputDecoration(labelText: 'Name'),
                ),
                TextField(
                  controller: _publicIdentityController,
                  minLines: 3,
                  maxLines: 8,
                  decoration: const InputDecoration(labelText: 'Public identity JSON'),
                ),
                const SizedBox(height: 8),
                FilledButton(
                  onPressed: () async {
                    await controller.addContact(
                      _contactNameController.text,
                      _publicIdentityController.text,
                    );
                    _contactNameController.clear();
                    _publicIdentityController.clear();
                  },
                  child: const Text('Add'),
                ),
                const Divider(height: 32),
                Text('Contacts', style: Theme.of(context).textTheme.titleMedium),
                for (final contact in contacts)
                  ListTile(
                    selected: contact.name == selectedContact,
                    title: Text(contact.name),
                    subtitle: Text(contact.peerId),
                    onTap: () => setState(() => _selectedContact = contact.name),
                  ),
              ],
            ),
          ),
          const VerticalDivider(width: 1),
          Expanded(
            child: Column(
              children: [
                Expanded(
                  child: ListView.builder(
                    padding: const EdgeInsets.all(16),
                    itemCount: messages.length,
                    itemBuilder: (context, index) {
                      final message = messages[index];
                      final outbound = message.direction == 'outbound';
                      return Align(
                        alignment: outbound ? Alignment.centerRight : Alignment.centerLeft,
                        child: Card(
                          child: Padding(
                            padding: const EdgeInsets.all(12),
                            child: Text(message.body),
                          ),
                        ),
                      );
                    },
                  ),
                ),
                Padding(
                  padding: const EdgeInsets.all(16),
                  child: Row(
                    children: [
                      Expanded(
                        child: TextField(
                          controller: _messageController,
                          decoration: const InputDecoration(labelText: 'Message'),
                        ),
                      ),
                      const SizedBox(width: 12),
                      FilledButton(
                        onPressed: selectedContact == null
                            ? null
                            : () async {
                                await controller.sendMessage(
                                  selectedContact,
                                  _messageController.text,
                                );
                                _messageController.clear();
                              },
                        child: const Text('Send'),
                      ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}
