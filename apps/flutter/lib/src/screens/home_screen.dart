import 'package:flutter/material.dart';

import '../models/chat_message.dart';
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
    if (mounted) {
      setState(() {});
    }
  }

  @override
  Widget build(BuildContext context) {
    final controller = widget.controller;

    return Scaffold(
      backgroundColor: Theme.of(context).colorScheme.surface,
      body: SafeArea(
        child: LayoutBuilder(
          builder: (context, constraints) {
            final compact = constraints.maxWidth < 820;
            final sidebar = _Sidebar(
              controller: controller,
              contactNameController: _contactNameController,
              publicIdentityController: _publicIdentityController,
            );
            final chat = _ChatPane(
              controller: controller,
              messageController: _messageController,
            );

            if (compact) {
              return Column(
                children: [
                  SizedBox(height: 360, child: sidebar),
                  const Divider(height: 1),
                  Expanded(child: chat),
                ],
              );
            }

            return Row(
              children: [
                SizedBox(width: 360, child: sidebar),
                const VerticalDivider(width: 1),
                Expanded(child: chat),
              ],
            );
          },
        ),
      ),
    );
  }
}

class _Sidebar extends StatelessWidget {
  const _Sidebar({
    required this.controller,
    required this.contactNameController,
    required this.publicIdentityController,
  });

  final MessengerController controller;
  final TextEditingController contactNameController;
  final TextEditingController publicIdentityController;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return DecoratedBox(
      decoration: BoxDecoration(
        color: theme.colorScheme.surfaceContainerHighest.withOpacity(0.35),
      ),
      child: ListView(
        padding: const EdgeInsets.all(20),
        children: [
          Row(
            children: [
              Container(
                width: 48,
                height: 48,
                decoration: BoxDecoration(
                  borderRadius: BorderRadius.circular(16),
                  gradient: LinearGradient(
                    colors: [
                      theme.colorScheme.primary,
                      theme.colorScheme.tertiary,
                    ],
                  ),
                ),
                child: const Icon(Icons.lock_rounded, color: Colors.white),
              ),
              const SizedBox(width: 12),
              Expanded(
                child: Column(
                  crossAxisAlignment: CrossAxisAlignment.start,
                  children: [
                    Text('Messenger', style: theme.textTheme.titleLarge),
                    Text(
                      'Encrypted relay demo',
                      style: theme.textTheme.bodySmall?.copyWith(
                        color: theme.colorScheme.onSurfaceVariant,
                      ),
                    ),
                  ],
                ),
              ),
              IconButton.filledTonal(
                onPressed: controller.busy ? null : controller.sync,
                icon: const Icon(Icons.sync_rounded),
                tooltip: 'Sync',
              ),
            ],
          ),
          const SizedBox(height: 20),
          _StatusCard(controller: controller),
          const SizedBox(height: 20),
          _AddContactCard(
            controller: controller,
            nameController: contactNameController,
            publicIdentityController: publicIdentityController,
          ),
          const SizedBox(height: 20),
          Text('Contacts', style: theme.textTheme.titleMedium),
          const SizedBox(height: 8),
          if (controller.contacts.isEmpty)
            const _EmptyState(
              icon: Icons.person_add_alt_1_rounded,
              title: 'No contacts yet',
              body: 'Paste a public identity JSON to start a conversation.',
            )
          else
            for (final contact in controller.contacts)
              Padding(
                padding: const EdgeInsets.only(bottom: 8),
                child: ListTile(
                  selected: contact.name == controller.selectedContact,
                  shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(16)),
                  leading: CircleAvatar(child: Text(_initialFor(contact.name))),
                  title: Text(contact.name),
                  subtitle: Text(
                    contact.peerId,
                    maxLines: 1,
                    overflow: TextOverflow.ellipsis,
                  ),
                  onTap: () => controller.selectContact(contact.name),
                ),
              ),
        ],
      ),
    );
  }
}

class _StatusCard extends StatelessWidget {
  const _StatusCard({required this.controller});

  final MessengerController controller;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final error = controller.error;

    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            Row(
              children: [
                Icon(
                  error == null ? Icons.verified_user_rounded : Icons.error_outline_rounded,
                  color: error == null ? theme.colorScheme.primary : theme.colorScheme.error,
                ),
                const SizedBox(width: 8),
                Text('Local identity', style: theme.textTheme.titleMedium),
              ],
            ),
            const SizedBox(height: 8),
            SelectableText(
              controller.peerId ?? 'Initializing...',
              style: theme.textTheme.bodySmall,
            ),
            if (controller.busy) ...[
              const SizedBox(height: 12),
              const LinearProgressIndicator(),
            ],
            if (error != null) ...[
              const SizedBox(height: 12),
              Text(error, style: TextStyle(color: theme.colorScheme.error)),
            ],
          ],
        ),
      ),
    );
  }
}

class _AddContactCard extends StatelessWidget {
  const _AddContactCard({
    required this.controller,
    required this.nameController,
    required this.publicIdentityController,
  });

  final MessengerController controller;
  final TextEditingController nameController;
  final TextEditingController publicIdentityController;

  @override
  Widget build(BuildContext context) {
    return Card(
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            Text('Add contact', style: Theme.of(context).textTheme.titleMedium),
            const SizedBox(height: 12),
            TextField(
              controller: nameController,
              textInputAction: TextInputAction.next,
              decoration: const InputDecoration(
                labelText: 'Display name',
                prefixIcon: Icon(Icons.badge_outlined),
              ),
            ),
            const SizedBox(height: 12),
            TextField(
              controller: publicIdentityController,
              minLines: 3,
              maxLines: 5,
              decoration: const InputDecoration(
                labelText: 'Public identity JSON',
                alignLabelWithHint: true,
                prefixIcon: Icon(Icons.key_rounded),
              ),
            ),
            const SizedBox(height: 12),
            FilledButton.icon(
              onPressed: controller.busy
                  ? null
                  : () async {
                      await controller.addContact(
                        name: nameController.text.trim(),
                        publicIdentityJson: publicIdentityController.text.trim(),
                      );
                      nameController.clear();
                      publicIdentityController.clear();
                    },
              icon: const Icon(Icons.person_add_alt_1_rounded),
              label: const Text('Add contact'),
            ),
          ],
        ),
      ),
    );
  }
}

class _ChatPane extends StatelessWidget {
  const _ChatPane({
    required this.controller,
    required this.messageController,
  });

  final MessengerController controller;
  final TextEditingController messageController;

  @override
  Widget build(BuildContext context) {
    final selectedContact = controller.selectedContact;
    final messages = selectedContact == null ? const <ChatMessage>[] : controller.messages;

    return Column(
      children: [
        _ChatHeader(controller: controller),
        const Divider(height: 1),
        Expanded(
          child: selectedContact == null
              ? const _EmptyState(
                  icon: Icons.forum_outlined,
                  title: 'Select a contact',
                  body: 'Choose a contact from the sidebar to start messaging.',
                )
              : messages.isEmpty
                  ? const _EmptyState(
                      icon: Icons.chat_bubble_outline_rounded,
                      title: 'No messages yet',
                      body: 'Send the first encrypted message in this conversation.',
                    )
                  : ListView.builder(
                      padding: const EdgeInsets.all(20),
                      itemCount: messages.length,
                      itemBuilder: (context, index) => _MessageBubble(message: messages[index]),
                    ),
        ),
        _Composer(
          enabled: selectedContact != null && !controller.busy,
          controller: messageController,
          onSend: () async {
            final body = messageController.text.trim();
            if (body.isEmpty) {
              return;
            }
            await controller.sendMessage(body);
            messageController.clear();
          },
        ),
      ],
    );
  }
}

class _ChatHeader extends StatelessWidget {
  const _ChatHeader({required this.controller});

  final MessengerController controller;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final selectedContact = controller.selectedContact;

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 16),
      child: Row(
        children: [
          CircleAvatar(child: Text(_initialFor(selectedContact))),
          const SizedBox(width: 12),
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(selectedContact ?? 'No conversation selected', style: theme.textTheme.titleMedium),
                Text(
                  'Relay-first encrypted messaging',
                  style: theme.textTheme.bodySmall?.copyWith(
                    color: theme.colorScheme.onSurfaceVariant,
                  ),
                ),
              ],
            ),
          ),
          FilledButton.tonalIcon(
            onPressed: controller.busy ? null : controller.sync,
            icon: const Icon(Icons.cloud_sync_rounded),
            label: const Text('Sync'),
          ),
        ],
      ),
    );
  }
}

class _MessageBubble extends StatelessWidget {
  const _MessageBubble({required this.message});

  final ChatMessage message;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final outbound = message.direction == MessageDirection.outbound;
    final bubbleColor = outbound ? theme.colorScheme.primary : theme.colorScheme.surfaceContainerHighest;
    final textColor = outbound ? theme.colorScheme.onPrimary : theme.colorScheme.onSurface;

    return Align(
      alignment: outbound ? Alignment.centerRight : Alignment.centerLeft,
      child: Container(
        constraints: const BoxConstraints(maxWidth: 560),
        margin: const EdgeInsets.only(bottom: 12),
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 12),
        decoration: BoxDecoration(
          color: bubbleColor,
          borderRadius: BorderRadius.only(
            topLeft: const Radius.circular(20),
            topRight: const Radius.circular(20),
            bottomLeft: Radius.circular(outbound ? 20 : 6),
            bottomRight: Radius.circular(outbound ? 6 : 20),
          ),
        ),
        child: Text(message.body, style: theme.textTheme.bodyLarge?.copyWith(color: textColor)),
      ),
    );
  }
}

class _Composer extends StatelessWidget {
  const _Composer({
    required this.enabled,
    required this.controller,
    required this.onSend,
  });

  final bool enabled;
  final TextEditingController controller;
  final Future<void> Function() onSend;

  @override
  Widget build(BuildContext context) {
    return DecoratedBox(
      decoration: BoxDecoration(
        color: Theme.of(context).colorScheme.surface,
        boxShadow: [
          BoxShadow(
            blurRadius: 18,
            color: Colors.black.withOpacity(0.06),
            offset: const Offset(0, -6),
          ),
        ],
      ),
      child: Padding(
        padding: const EdgeInsets.all(16),
        child: Row(
          children: [
            Expanded(
              child: TextField(
                controller: controller,
                enabled: enabled,
                minLines: 1,
                maxLines: 4,
                decoration: const InputDecoration(
                  hintText: 'Write an encrypted message...',
                  prefixIcon: Icon(Icons.lock_outline_rounded),
                ),
                onSubmitted: enabled ? (_) => onSend() : null,
              ),
            ),
            const SizedBox(width: 12),
            FilledButton.icon(
              onPressed: enabled ? onSend : null,
              icon: const Icon(Icons.send_rounded),
              label: const Text('Send'),
            ),
          ],
        ),
      ),
    );
  }
}

class _EmptyState extends StatelessWidget {
  const _EmptyState({
    required this.icon,
    required this.title,
    required this.body,
  });

  final IconData icon;
  final String title;
  final String body;

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 360),
        child: Padding(
          padding: const EdgeInsets.all(32),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Icon(icon, size: 56, color: theme.colorScheme.primary),
              const SizedBox(height: 16),
              Text(title, style: theme.textTheme.titleLarge, textAlign: TextAlign.center),
              const SizedBox(height: 8),
              Text(
                body,
                style: theme.textTheme.bodyMedium?.copyWith(
                  color: theme.colorScheme.onSurfaceVariant,
                ),
                textAlign: TextAlign.center,
              ),
            ],
          ),
        ),
      ),
    );
  }
}

String _initialFor(String? value) {
  final trimmed = value?.trim() ?? '';
  if (trimmed.isEmpty) {
    return '?';
  }
  return trimmed.substring(0, 1).toUpperCase();
}
