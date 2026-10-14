import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mxmdaw/engine/audio.dart';
import 'package:mxmdaw/engine/engine.dart';
import 'package:mxmdaw/main.dart';

/// Mesin palsu: mencatat panggilan, tanpa perangkat audio.
class MesinPalsu implements MesinAudio {
  MesinPalsu({this.galatMulai});

  final Object? galatMulai;
  final ketukan = <(int, double)>[];
  final lepasan = <int>[];
  var berbunyi = 0;
  var mulaiDipanggil = 0;
  var berhentiDipanggil = 0;

  @override
  Future<void> mulai() async {
    mulaiDipanggil++;
    if (galatMulai != null) throw galatMulai!;
  }

  @override
  Future<void> berhenti() async => berhentiDipanggil++;

  @override
  bool ketuk(int pad, double velocity) {
    ketukan.add((pad, velocity));
    return true;
  }

  @override
  bool lepas(int pad) {
    lepasan.add(pad);
    return true;
  }

  @override
  int padBerbunyi() => berbunyi;

  final nama = [for (var i = 1; i <= 16; i++) 'SUARA$i'];

  @override
  List<String> namaPad() => List.of(nama);

  final impor = <(int, String, int)>[];
  String? galatImpor;

  @override
  Future<PadSettings> imporWav(
    int pad,
    String namaBerkas,
    Uint8List isi,
  ) async {
    if (galatImpor != null) throw galatImpor!;
    impor.add((pad, namaBerkas, isi.length));
    nama[pad] = namaBerkas.toUpperCase().split('.').first;
    return pengaturanPad[pad] = pengaturanBawaan.salin(end: isi.length);
  }

  final pengaturanPad = List.filled(16, pengaturanBawaan);
  final perubahan = <(int, PadSettings)>[];

  @override
  PadSettings? pengaturan(int pad) => pengaturanPad[pad];

  @override
  PadSettings? aturPengaturan(int pad, PadSettings nilai) {
    perubahan.add((pad, nilai));
    return pengaturanPad[pad] = nilai;
  }

  @override
  (int, int) infoSampel(int pad) => (48000, 48000);

  @override
  Float32List puncak(int pad, int titik) => Float32List.fromList([
    for (var i = 0; i < titik; i++) ...[-0.5, 0.5],
  ]);

  @override
  AudioHealth kesehatan() => const AudioHealth(
    running: true,
    sampleRate: 48000,
    framesPerBurst: 96,
    bufferSize: 192,
    xruns: 0,
    latencyMs: 12.34,
    loadAvg: 0.031,
    loadMax: 0.094,
    exclusive: true,
    aaudio: true,
    restarts: 0,
    voicesActive: 1,
    hitsPlayed: 7,
    hitsDropped: 0,
  );
}

const pengaturanBawaan = PadSettings(
  level: 1,
  pan: 0,
  pitch: 0,
  start: 0,
  end: 48000,
  attack: 0.001,
  decay: 0,
  sustain: 1,
  release: 0.02,
  filter: FilterKind.off,
  cutoff: 1,
  resonance: 0,
  drive: 0,
  play: PlayKind.oneShot,
  choke: 0,
);

Future<MesinPalsu> pasang(WidgetTester tester, {Object? galatMulai}) async {
  // Layar HP kecil (360 × 740 dp), seperti Redmi 13C.
  tester.view.physicalSize = const Size(1080, 2220);
  tester.view.devicePixelRatio = 3;
  addTearDown(tester.view.reset);
  final mesin = MesinPalsu(galatMulai: galatMulai);
  await tester.pumpWidget(
    MxmDawApp(engine: const EngineReady('0.1.0'), audio: mesin),
  );
  await tester.pump();
  return mesin;
}

void main() {
  testWidgets('stream dimulai dan panel menampilkan angka kesehatan', (
    tester,
  ) async {
    final mesin = await pasang(tester);

    expect(mesin.mulaiDipanggil, 1);
    expect(find.text('LAT 12.3 ms   BUF 192/96'), findsOneWidget);
    expect(find.text('BEBAN 3% (MAKS 9%)   XRUN 0'), findsOneWidget);
    expect(find.text('48000 Hz · AAudio · EKSKLUSIF'), findsOneWidget);
    expect(find.text('SUARA 1   KETUK 7'), findsOneWidget);
  });

  testWidgets('16 pad dengan nama suara dari mesin', (tester) async {
    await pasang(tester);

    for (var i = 0; i < 16; i++) {
      expect(find.byKey(ValueKey('pad-$i')), findsOneWidget);
    }
    expect(find.text('SUARA1'), findsOneWidget);
    expect(find.text('SUARA16'), findsOneWidget);
  });

  testWidgets('pad muat di layar 360 dp dan tidak lebih kecil dari 48 dp', (
    tester,
  ) async {
    await pasang(tester);

    final pad = tester.getSize(find.byKey(const ValueKey('pad-0')));
    expect(pad.width, greaterThanOrEqualTo(48));
    expect(pad.width, lessThanOrEqualTo(72));
    expect(tester.takeException(), isNull, reason: 'tanpa overflow');
  });

  testWidgets('ketukan di atas pad = velocity tinggi, di bawah = rendah', (
    tester,
  ) async {
    final mesin = await pasang(tester);
    final pad = tester.getRect(find.byKey(const ValueKey('pad-5')));

    await tester.tapAt(pad.topCenter + const Offset(0, 2));
    await tester.tapAt(pad.bottomCenter - const Offset(0, 2));
    await tester.pump(const Duration(milliseconds: 200));

    expect(mesin.ketukan, hasLength(2));
    expect(mesin.ketukan[0].$1, 5);
    expect(mesin.ketukan[0].$2, closeTo(1.0, 0.03));
    expect(mesin.ketukan[1].$2, closeTo(0.4, 0.03));
    expect(mesin.lepasan, [5, 5]);
  });

  testWidgets('dua jari di dua pad: keduanya berbunyi, lepas masing-masing', (
    tester,
  ) async {
    final mesin = await pasang(tester);
    final a = tester.getCenter(find.byKey(const ValueKey('pad-0')));
    final b = tester.getCenter(find.byKey(const ValueKey('pad-15')));

    final jari1 = await tester.startGesture(a, pointer: 1);
    final jari2 = await tester.startGesture(b, pointer: 2);
    expect(mesin.ketukan.map((k) => k.$1), [0, 15]);
    expect(mesin.lepasan, isEmpty, reason: 'masih ditekan');

    await jari2.up();
    expect(mesin.lepasan, [15]);
    await jari1.up();
    expect(mesin.lepasan, [15, 0]);
  });

  testWidgets('LED menyala mengikuti pad yang berbunyi di mesin', (
    tester,
  ) async {
    final mesin = await pasang(tester);
    Color warnaLed(int i) =>
        ((tester.widget<AnimatedContainer>(find.byKey(ValueKey('led-$i'))))
                    .decoration!
                as BoxDecoration)
            .color!;
    final padam = warnaLed(2);

    mesin.berbunyi = 1 << 2;
    await tester.pump(const Duration(milliseconds: 40));
    expect(warnaLed(2), isNot(padam));
    expect(warnaLed(3), padam);

    mesin.berbunyi = 0;
    await tester.pump(const Duration(milliseconds: 40));
    expect(warnaLed(2), padam);
  });

  testWidgets('tekan lama pad membuka halaman Sound pad itu', (tester) async {
    final mesin = await pasang(tester);
    final pad = tester.getCenter(find.byKey(const ValueKey('pad-6')));

    final jari = await tester.startGesture(pad);
    await tester.pump(const Duration(milliseconds: 600));
    await tester.pumpAndSettle();
    await jari.up();

    expect(find.text('A:07  SUARA7'), findsOneWidget);
    expect(mesin.lepasan, contains(6), reason: 'pad dilepas sebelum pindah');
  });

  testWidgets('ketukan singkat tidak membuka halaman Sound', (tester) async {
    await pasang(tester);
    await tester.tap(find.byKey(const ValueKey('pad-6')));
    await tester.pump(const Duration(milliseconds: 600));
    await tester.pumpAndSettle();

    expect(find.text('A:07  SUARA7'), findsNothing);
  });

  testWidgets('aplikasi ke latar → stream berhenti; kembali → mulai lagi', (
    tester,
  ) async {
    final mesin = await pasang(tester);

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.paused);
    await tester.pump();
    expect(mesin.berhentiDipanggil, greaterThanOrEqualTo(1));

    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.hidden);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.inactive);
    tester.binding.handleAppLifecycleStateChanged(AppLifecycleState.resumed);
    await tester.pump();
    expect(mesin.mulaiDipanggil, 2);
  });

  testWidgets('stream gagal dibuka: galatnya tampil, aplikasi tetap hidup', (
    tester,
  ) async {
    await pasang(tester, galatMulai: 'kode Oboe -896');

    expect(find.textContaining('AUDIO GAGAL: kode Oboe -896'), findsOneWidget);
  });

  testWidgets('tanpa mesin: pad nonaktif, ketukan tidak dikirim', (
    tester,
  ) async {
    await tester.pumpWidget(
      const MxmDawApp(engine: EngineUnavailable('pustaka tidak ditemukan')),
    );

    await tester.tap(find.byKey(const ValueKey('pad-0')), warnIfMissed: false);
    expect(find.text('AUDIO TIDAK TERSEDIA'), findsOneWidget);
    expect(find.text('MESIN TIDAK TERMUAT'), findsOneWidget);
  });
}
