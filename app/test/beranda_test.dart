import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mxmdaw/engine/audio.dart';
import 'package:mxmdaw/engine/engine.dart';
import 'package:mxmdaw/main.dart';

/// Mesin palsu: mencatat panggilan, tanpa perangkat audio.
class MesinPalsu implements MesinAudio {
  MesinPalsu({this.galatMulai});

  final Object? galatMulai;
  final ketukan = <double>[];
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
  bool ketuk(double velocity) {
    ketukan.add(velocity);
    return true;
  }

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

Future<MesinPalsu> pasang(WidgetTester tester, {Object? galatMulai}) async {
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

  testWidgets('ketukan di atas pad = velocity tinggi, di bawah = rendah', (
    tester,
  ) async {
    final mesin = await pasang(tester);
    final pad = tester.getRect(find.byKey(const ValueKey('pad-kick')));

    await tester.tapAt(pad.topCenter + const Offset(0, 2));
    await tester.tapAt(pad.bottomCenter - const Offset(0, 2));
    await tester.pump(const Duration(milliseconds: 200));

    expect(mesin.ketukan, hasLength(2));
    expect(mesin.ketukan[0], closeTo(1.0, 0.02));
    expect(mesin.ketukan[1], closeTo(0.4, 0.02));
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

    await tester.tap(find.byKey(const ValueKey('pad-kick')));
    expect(find.text('AUDIO TIDAK TERSEDIA'), findsOneWidget);
    expect(find.text('MESIN TIDAK TERMUAT'), findsOneWidget);
  });
}
