import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mxmdaw/app/theme.dart';
import 'package:mxmdaw/design_system/kenop.dart';
import 'package:mxmdaw/design_system/tokens.dart';
import 'package:mxmdaw/engine/audio.dart';
import 'package:mxmdaw/engine/papan_klip.dart';
import 'package:mxmdaw/features/pola/halaman_pola.dart';

import '../../beranda_test.dart' show MesinPalsu;

Future<MesinPalsu> buka(
  WidgetTester tester, {
  int bars = 1,
  int grup = 0,
  PapanKlip? klip,
}) async {
  tester.view.physicalSize = const Size(1080, 2220);
  tester.view.devicePixelRatio = 3;
  addTearDown(tester.view.reset);
  final mesin = MesinPalsu();
  mesin.seq = mesin.seq.salin(bars: bars);
  await tester.pumpWidget(
    MaterialApp(
      theme: mxmTheme(MxmTokens.light, Brightness.light),
      home: HalamanPola(
        audio: mesin,
        grup: grup,
        nama: mesin.namaPad().sublist(grup * 16, grup * 16 + 16),
        pad: 1,
        klip: klip,
      ),
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
      ..not = [
        const NoteData(tick: 245, pad: 1, velocity: 0.9, length: 240, pitch: 0),
      ]
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
    expect(find.text('A · 1 BAR  ·  PAD 2 SUARA2'), findsOneWidget);
    await tester.tap(find.byKey(const ValueKey('pilih-pad-0')));
    await tester.pump();
    expect(find.text('A · 1 BAR  ·  PAD 1 SUARA1'), findsOneWidget);
  });

  testWidgets('tombol main di halaman Pola', (tester) async {
    final mesin = await buka(tester);
    await tester.tap(find.byKey(const ValueKey('pola-main')));
    expect(mesin.perintah, ['main']);
  });

  Future<void> pilihMenu(WidgetTester tester, String teks) async {
    await tester.tap(find.byKey(const ValueKey('menu-pola')));
    await tester.pumpAndSettle();
    await tester.tap(find.text(teks));
    await tester.pumpAndSettle();
  }

  testWidgets('SALIN POLA menyimpan not dan panjang ke papan klip', (
    tester,
  ) async {
    final klip = PapanKlip();
    await buka(tester, bars: 2, klip: klip);
    await tester.tap(find.byKey(const ValueKey('langkah-4')));
    await tester.pump();
    await pilihMenu(tester, 'SALIN POLA');

    expect(klip.pola!.bars, 2);
    expect(klip.pola!.not.single.tick, 960);
    expect(find.text('Pola A disalin'), findsOneWidget);
  });

  testWidgets('TEMPEL POLA ke grup lain: panjang dulu, lalu not', (
    tester,
  ) async {
    const n = NoteData(tick: 3000, pad: 4, velocity: 1, length: 240, pitch: 0);
    final klip = PapanKlip()..pola = (not: [n], bars: 4);
    final mesin = await buka(tester, grup: 2, klip: klip);
    await pilihMenu(tester, 'TEMPEL POLA');

    expect(mesin.seq.bars, 4);
    expect(mesin.grupPola, 2);
    expect(mesin.not, [n]);
    expect(find.textContaining('C · 4 BAR'), findsOneWidget);
  });

  testWidgets('TEMPEL GRUP menyalin suara + pola dan memperbarui nama', (
    tester,
  ) async {
    final klip = PapanKlip()..grup = 0;
    final mesin = await buka(tester, grup: 1, klip: klip);
    expect(find.textContaining('B2'), findsWidgets);
    await pilihMenu(tester, 'TEMPEL GRUP');

    expect(mesin.perintah, ['salin-grup 0→1']);
    expect(find.textContaining('SUARA2'), findsWidgets);
    expect(find.text('Grup A ditempel ke B'), findsOneWidget);
  });

  testWidgets('TEMPEL GRUP ke grup yang sama tidak bisa dipilih', (
    tester,
  ) async {
    final klip = PapanKlip()..grup = 0;
    final mesin = await buka(tester, klip: klip);
    await pilihMenu(tester, 'TEMPEL GRUP');
    expect(mesin.perintah, isEmpty);
  });
}
