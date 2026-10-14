import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mxmdaw/app/theme.dart';
import 'package:mxmdaw/design_system/kenop.dart';
import 'package:mxmdaw/design_system/tokens.dart';

import 'dart:typed_data';

import 'package:mxmdaw/engine/audio.dart';
import 'package:mxmdaw/engine/berkas.dart';
import 'package:mxmdaw/features/suara/halaman_suara.dart';

import '../../beranda_test.dart' show MesinPalsu;

class PemilihPalsu implements PemilihBerkas {
  PemilihPalsu(this.hasil);

  final BerkasAudio? Function() hasil;

  @override
  Future<BerkasAudio?> pilihAudio() async => hasil();
}

Future<MesinPalsu> buka(WidgetTester tester, {PemilihBerkas? pemilih}) async {
  tester.view.physicalSize = const Size(1080, 2220);
  tester.view.devicePixelRatio = 3;
  addTearDown(tester.view.reset);
  final mesin = MesinPalsu();
  await tester.pumpWidget(
    MaterialApp(
      theme: mxmTheme(MxmTokens.light, Brightness.light),
      home: HalamanSuara(
        audio: mesin,
        pad: 2,
        nama: 'CLAP',
        pemilih: pemilih ?? PemilihPalsu(() => null),
      ),
    ),
  );
  return mesin;
}

Finder kenop(String label) =>
    find.ancestor(of: find.text(label), matching: find.byType(Kenop));

void main() {
  testWidgets('judul menyebut grup, nomor, dan nama pad', (tester) async {
    await buka(tester);
    expect(find.text('A:03  CLAP'), findsOneWidget);
  });

  testWidgets('geser kenop LEVEL ke bawah 100 dp = level turun separuh', (
    tester,
  ) async {
    final mesin = await buka(tester);
    await tester.drag(kenop('LEVEL'), const Offset(0, 100));
    // Tunggu sampai pengenal ketuk-ganda menyerah.
    await tester.pump(const Duration(milliseconds: 400));

    final (pad, p) = mesin.perubahan.last;
    expect(pad, 2);
    expect(p.level, closeTo(0.5, 0.05));
    expect(find.text('${(p.level * 100).round()}%'), findsOneWidget);
  });

  testWidgets('ketuk ganda kenop mengembalikan nilai bawaan', (tester) async {
    final mesin = await buka(tester);
    await tester.drag(kenop('PITCH'), const Offset(0, -100));
    await tester.pump();
    expect(mesin.pengaturanPad[2].pitch, isNot(0));

    await tester.tap(kenop('PITCH'));
    await tester.pump(const Duration(milliseconds: 50));
    await tester.tap(kenop('PITCH'));
    await tester.pumpAndSettle();
    expect(mesin.pengaturanPad[2].pitch, 0);
  });

  testWidgets('pilih filter LP, mode LOOP, dan choke grup 1', (tester) async {
    final mesin = await buka(tester);

    await tester.tap(find.byKey(const ValueKey('filter-LP')));
    await tester.pump();
    await tester.scrollUntilVisible(
      find.byKey(const ValueKey('choke-tambah')),
      200,
    );
    await tester.tap(find.byKey(const ValueKey('mode-LOOP')));
    await tester.pump();
    await tester.tap(find.byKey(const ValueKey('choke-tambah')));
    await tester.pump();

    final p = mesin.pengaturanPad[2];
    expect(p.filter, FilterKind.lowPass);
    expect(p.play, PlayKind.loop);
    expect(p.choke, 1);
    expect(find.text('GRUP 1'), findsOneWidget);
  });

  testWidgets('geser penanda start pada gelombang', (tester) async {
    final mesin = await buka(tester);
    final g = tester.getRect(find.byKey(const ValueKey('gelombang')));

    await tester.dragFrom(
      g.centerLeft + const Offset(2, 0),
      Offset(g.width / 4, 0),
    );
    await tester.pump();

    final p = mesin.pengaturanPad[2];
    expect(p.start, closeTo(12000, 1000));
    expect(p.end, 48000);
  });

  testWidgets('tombol dengar: tekan = ketuk, angkat = lepas', (tester) async {
    final mesin = await buka(tester);
    final jari = await tester.startGesture(
      tester.getCenter(find.byKey(const ValueKey('dengar'))),
    );
    expect(mesin.ketukan.single.$1, 2);
    await jari.up();
    expect(mesin.lepasan, [2]);
  });

  testWidgets('impor WAV: nama pad dan panjang sampel diperbarui', (
    tester,
  ) async {
    final mesin = await buka(
      tester,
      pemilih: PemilihPalsu(
        () => BerkasAudio(nama: 'vokal.wav', isi: Uint8List(1234)),
      ),
    );
    await tester.tap(find.byKey(const ValueKey('impor')));
    await tester.pumpAndSettle();

    expect(mesin.impor.single, (2, 'vokal.wav', 1234));
    expect(find.text('A:03  VOKAL'), findsOneWidget);
    expect(find.textContaining('/ 48000'), findsOneWidget);
    expect(find.text('Diimpor: vokal.wav'), findsOneWidget);
  });

  testWidgets('impor dibatalkan: mesin tidak disentuh', (tester) async {
    final mesin = await buka(tester);
    await tester.tap(find.byKey(const ValueKey('impor')));
    await tester.pumpAndSettle();

    expect(mesin.impor, isEmpty);
    expect(find.text('A:03  CLAP'), findsOneWidget);
  });

  testWidgets('impor gagal: pesan dari Rust tampil, halaman tetap hidup', (
    tester,
  ) async {
    final mesin = await buka(
      tester,
      pemilih: PemilihPalsu(
        () => BerkasAudio(nama: 'lagu.mp3', isi: Uint8List(10)),
      ),
    );
    mesin.galatImpor = 'bukan berkas WAV yang bisa dibaca';
    await tester.tap(find.byKey(const ValueKey('impor')));
    await tester.pumpAndSettle();

    expect(
      find.text('Impor gagal: bukan berkas WAV yang bisa dibaca'),
      findsOneWidget,
    );
    expect(find.text('A:03  CLAP'), findsOneWidget);
  });
}
