import 'package:flutter/material.dart';

import 'bridge/messenger_bridge.dart';
import 'bridge/mock_messenger_bridge.dart';
import 'screens/home_screen.dart';
import 'state/messenger_controller.dart';

class MessengerApp extends StatelessWidget {
  MessengerApp({super.key, MessengerBridge? bridge})
      : bridge = bridge ?? MockMessengerBridge();

  final MessengerBridge bridge;

  @override
  Widget build(BuildContext context) {
    final controller = MessengerController(
      bridge: bridge,
      config: const ClientConfig(
        databasePath: 'messenger-client.db',
        relayUrl: 'http://127.0.0.1:8080',
      ),
    );

    return MaterialApp(
      title: 'Messenger',
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(seedColor: Colors.indigo),
        useMaterial3: true,
      ),
      home: HomeScreen(controller: controller),
    );
  }
}
