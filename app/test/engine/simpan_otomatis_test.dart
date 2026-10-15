import 'dart:async';

import 'package:flutter/widgets.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mxmdaw/engine/proyek.dart';
import 'package:mxmdaw/engine/simpan_otomatis.dart';

/// Proyek palsu: revisi diatur tes; simpan bisa ditahan atau digagalkan.
class ProyekPalsu implements Proyek {
  int rev = 0;
  int jumlahSimpan = 0;
  Object? galat;
  Completer<void>? tahan;

  @override
  Future<List<String>> buka() async => const [];

  @override
  Future<void> simpan() async {
    jumlahSimpan++;
    await tahan?.future;
    if (galat != null) throw galat!;
  }

  @override
  int revisi() => rev;
}

void main() {
  // Timer dijalankan lewat jam palsu milik testWidgets.
  Future<void> jalan(WidgetTester tester, Duration d) => tester.pump(d);
  const jeda = Duration(milliseconds: 500);

  testWidgets('tanpa perubahan: tidak menyimpan', (tester) async {
    await tester.pumpWidget(const SizedBox());
    final p = ProyekPalsu()..rev = 7;
    final s = SimpanOtomatis(p)..mulai();
    await jalan(tester, jeda * 4);
    expect(p.jumlahSimpan, 0, reason: 'revisi saat mulai = sudah tersimpan');
    s.berhenti();
  });

  testWidgets('perubahan disimpan sekali dalam satu jeda', (tester) async {
    await tester.pumpWidget(const SizedBox());
    final p = ProyekPalsu();
    final s = SimpanOtomatis(p)..mulai();
    p.rev = 1;
    await jalan(tester, jeda);
    expect(p.jumlahSimpan, 1);
    await jalan(tester, jeda * 3);
    expect(p.jumlahSimpan, 1, reason: 'tidak ada perubahan baru');
    p.rev = 2;
    await jalan(tester, jeda);
    expect(p.jumlahSimpan, 2);
    s.berhenti();
  });

  testWidgets('simpan yang lambat tidak ditumpuk', (tester) async {
    await tester.pumpWidget(const SizedBox());
    final p = ProyekPalsu()..tahan = Completer();
    final s = SimpanOtomatis(p)..mulai();
    p.rev = 1;
    await jalan(tester, jeda);
    p.rev = 2;
    await jalan(tester, jeda * 4);
    expect(p.jumlahSimpan, 1, reason: 'masih menyimpan revisi 1');
    p.tahan!.complete();
    p.tahan = null;
    await jalan(tester, jeda);
    expect(p.jumlahSimpan, 2, reason: 'revisi 2 menyusul');
    s.berhenti();
  });

  testWidgets('gagal: dilaporkan lalu dicoba lagi', (tester) async {
    await tester.pumpWidget(const SizedBox());
    final galat = <Object>[];
    var berhasil = 0;
    final p = ProyekPalsu()..galat = 'disk penuh';
    final s = SimpanOtomatis(
      p,
      onGalat: galat.add,
      onTersimpan: () => berhasil++,
    )..mulai();
    p.rev = 1;
    await jalan(tester, jeda);
    expect(galat, ['disk penuh']);
    p.galat = null;
    await jalan(tester, jeda);
    expect(berhasil, 1);
    expect(p.jumlahSimpan, 2);
    s.berhenti();
  });

  testWidgets('simpanSekarang tanpa menunggu jeda', (tester) async {
    await tester.pumpWidget(const SizedBox());
    final p = ProyekPalsu();
    final s = SimpanOtomatis(p)..mulai();
    p.rev = 3;
    await s.simpanSekarang();
    expect(p.jumlahSimpan, 1);
    s.berhenti();
  });
}
