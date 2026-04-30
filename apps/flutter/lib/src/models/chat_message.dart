enum MessageDirection {
  inbound,
  outbound,
}

class ChatMessage {
  const ChatMessage({
    required this.messageId,
    required this.contactName,
    required this.direction,
    required this.body,
    required this.createdAtMs,
  });

  final String messageId;
  final String contactName;
  final MessageDirection direction;
  final String body;
  final int createdAtMs;
}
