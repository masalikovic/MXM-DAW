import 'package:flutter_test/flutter_test.dart';
import 'package:mxmdaw/main.dart';

void main() {
  testWidgets('layar pertama menampilkan nama aplikasi', (tester) async {
    await tester.pumpWidget(const MxmDawApp());

    expect(find.text('MXM-DAW'), findsOneWidget);
  });
}
