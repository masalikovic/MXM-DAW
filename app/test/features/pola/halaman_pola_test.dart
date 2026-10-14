import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mxmdaw/app/theme.dart';
import 'package:mxmdaw/design_system/kenop.dart';
import 'package:mxmdaw/design_system/tokens.dart';
import 'package:mxmdaw/engine/audio.dart';
import 'package:mxmdaw/features/pola/halaman_pola.dart';

import '../../beranda_test.dart' show MesinPalsu;

Future<MesinPalsu> buka(WidgetTester tester, {int bars = 1}) async {
  tester.view.physicalSize = const Size(1080, 2220);
  tester.view.devicePixelRatio = 3;
  addTearDown(tester.view.reset);
  final mesin = MesinPalsu();
  mesin.seq = mesin.seq.salin(bars: bars);
  await tester.pumpWidget(
    MaterialApp(
      theme: mxmTheme(MxmTokens.light, Brightness.light),
      home: HalamanPola(audio: mesin, nama: mesin.namaPad(), pad: 1),
    ),
  );
  return mesin;
}

void main() {
  testWidgets('ketuk langkah menyalakan not untuk pad terpilih', (
    tester,
  ) async {
    final mesin = await buka(tester);
    await tester.tap(find.byKey(const ValueKey('langkah-4')));
    await tester.pump();

    expect(mesin.not.single.pad, 1);
    expect(mesin.not.single.tick, 960);
    expect(find.byKey(const ValueKey('isi-4')), findsOneWidget);

    await tester.tap(find.byKey(const ValueKey('langkah-4')));
    await tester.pump();
    expect(mesin.not, isEmpty);
  });

  testWidgets('ganti pad dan halaman bar', (tester) async {
    final mesin = await buka(tester, bars: 2);
    await tester.tap(find.byKey(const ValueKey('pilih-pad-0')));
    await tester.tap(find.byKey(const ValueKey('halaman-1')));
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('langkah-0')));
    await tester.pump();

    expect(mesin.not.single.pad, 0);
    expect(mesin.not.single.tick, 3840, reason: 'langkah 1 bar 2');
  });

  testWidgets('tahan langkah memutar velocity', (tester) async {
    final mesin = await buka(tester);
    await tester.tap(find.byKey(const ValueKey('langkah-0')));
    await tester.pump();
    await tester.longPress(find.byKey(const ValueKey('langkah-0')));
    await tester.pump();
    expect(mesin.not.single.velocity, 1.0);
  });

  testWidgets('rekaman yang masuk saat halaman terbuka langsung tampil', (
    tester,
  ) async {
    final mesin = await buka(tester);
    mesin
      ..not = [const NoteData(tick: 245, pad: 1, velocity: 0.9, length: 240)]
      ..status = const TransportState(
        playing: true,
        recording: true,
        positionTicks: 300,
        bar: 1,
        beat: 1,
        step: 1,
        noteCount: 1,
      );
    await tester.pump(const Duration(milliseconds: 40));
    expect(find.byKey(const ValueKey('isi-1')), findsOneWidget);
  });

  testWidgets('grid kuantisasi, kekuatan, dan swing', (tester) async {
    final mesin = await buka(tester);
    await tester.scrollUntilVisible(
      find.text('SWING'),
      200,
      // Halaman (vertikal), bukan baris chip pad (horizontal).
      scrollable: find.byType(Scrollable).first,
    );
    await tester.tap(find.byKey(const ValueKey('grid-1/8')));
    await tester.pump();
    final kenop = find.ancestor(
      of: find.text('SWING'),
      matching: find.byType(Kenop),
    );
    await tester.drag(kenop, const Offset(0, -100)); // separuh = 62,5 %
    await tester.pump(const Duration(milliseconds: 400));

    expect(mesin.seq.quantizeGrid, 480);
    expect(mesin.seq.swing, closeTo(0.625, 0.01));
  });

  testWidgets('judul menyebut pad yang sedang diedit', (tester) async {
    await buka(tester);
    expect(find.text('1 BAR  ·  PAD 2 SUARA2'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('pilih-pad-0')));
    await tester.pump();
    expect(find.text('1 BAR  ·  PAD 1 SUARA1'), findsOneWidget);
  });

  testWidgets('tombol main di halaman Pola', (tester) async {
    final mesin = await buka(tester);
    await tester.tap(find.byKey(const ValueKey('pola-main')));
    expect(mesin.perintah, ['main']);
  });
}
