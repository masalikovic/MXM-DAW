import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mxmdaw/design_system/tokens.dart';
import 'package:mxmdaw/engine/engine.dart';
import 'package:mxmdaw/main.dart';

void main() {
  testWidgets('layar pertama menampilkan nama aplikasi', (tester) async {
    await tester.pumpWidget(const MxmDawApp(engine: EngineReady('0.1.0')));

    expect(find.text('MXM-DAW'), findsOneWidget);
  });

  testWidgets('versi mesin dari Rust tampil di layar', (tester) async {
    await tester.pumpWidget(const MxmDawApp(engine: EngineReady('9.8.7')));

    expect(find.text('MESIN 9.8.7'), findsOneWidget);
  });

  testWidgets('mesin gagal dimuat: alasannya tampil, aplikasi tetap hidup', (
    tester,
  ) async {
    await tester.pumpWidget(
      const MxmDawApp(engine: EngineUnavailable('pustaka tidak ditemukan')),
    );

    expect(find.text('MESIN TIDAK TERMUAT'), findsOneWidget);
    expect(find.text('pustaka tidak ditemukan'), findsOneWidget);
    expect(find.textContaining('MESIN 0'), findsNothing);
  });

  for (final (kecerahan, token) in [
    (Brightness.light, MxmTokens.light),
    (Brightness.dark, MxmTokens.dark),
  ]) {
    testWidgets('HP bertema ${kecerahan.name}: panel dan teks memakai token', (
      tester,
    ) async {
      tester.platformDispatcher.platformBrightnessTestValue = kecerahan;
      addTearDown(tester.platformDispatcher.clearPlatformBrightnessTestValue);

      await tester.pumpWidget(const MxmDawApp(engine: EngineReady('0.1.0')));

      final scaffold = tester.widget<Scaffold>(find.byType(Scaffold));
      final latar =
          scaffold.backgroundColor ??
          Theme.of(tester.element(find.byType(Scaffold)))
              .scaffoldBackgroundColor;
      expect(latar, token.body);

      final judul = tester.widget<Text>(find.text('MXM-DAW'));
      expect(judul.style?.color, token.legendOnLight);
    });
  }
}
