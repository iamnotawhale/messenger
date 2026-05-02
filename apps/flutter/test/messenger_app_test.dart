import 'package:flutter_test/flutter_test.dart';
import 'package:messenger_app/src/app.dart';

void main() {
  testWidgets('renders the mock-backed messenger shell', (tester) async {
    await tester.pumpWidget(MessengerApp());
    await tester.pump();

    expect(find.text('Messenger'), findsOneWidget);
    expect(find.text('Secure peer-to-peer relay messenger'), findsOneWidget);
    expect(find.text('Add secure contact'), findsOneWidget);
  });
}
