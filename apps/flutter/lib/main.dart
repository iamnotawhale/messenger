import 'package:flutter/material.dart';

import 'src/app.dart';
import 'src/bridge/messenger_bridge_factory.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final bridge = await createDefaultMessengerBridge();
  runApp(MessengerApp(bridge: bridge));
}
