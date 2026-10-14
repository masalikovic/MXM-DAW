import 'dart:math' as math;

/// Pemetaan nilai kenop 0..1 ↔ satuan parameter (rencana §10), dan cara
/// menampilkannya. Fungsi murni, jadi mudah dites.
abstract final class Pemetaan {
  /// Waktu amplop: 1 ms … 10 s, eksponensial seperti telinga.
  static double waktu(double x) => 0.001 * math.pow(10000, x.clamp(0.0, 1.0));

  static double waktuKeKenop(double detik) =>
      (math.log(detik.clamp(0.001, 10.0) / 0.001) / math.log(10000)).clamp(
        0.0,
        1.0,
      );

  /// Pitch utuh −24..24 semitone.
  static int semitone(double x) => (-24 + 48 * x.clamp(0.0, 1.0)).round();

  static double semitoneKeKenop(int st) => ((st + 24) / 48).clamp(0.0, 1.0);

  /// Cent −50..50.
  static int cent(double x) => (-50 + 100 * x.clamp(0.0, 1.0)).round();

  static double centKeKenop(int c) => ((c + 50) / 100).clamp(0.0, 1.0);

  /// Pitch tersimpan (pecahan = cent) → (semitone, cent).
  static (int, int) pecahPitch(double pitch) {
    final st = pitch.round();
    return (st, ((pitch - st) * 100).round());
  }

  /// Pan −1..1.
  static double pan(double x) => -1 + 2 * x.clamp(0.0, 1.0);

  static double panKeKenop(double p) => ((p + 1) / 2).clamp(0.0, 1.0);

  /// Cutoff 0..1 → Hz, sama dengan `daw_core::dsp::cutoff_ke_hz`.
  static double cutoffHz(double x) => 20.0 * math.pow(1000, x.clamp(0.0, 1.0));

  static String teksWaktu(double detik) => detik < 1
      ? '${(detik * 1000).round()} ms'
      : '${detik.toStringAsFixed(2)} s';

  static String teksPersen(double x) => '${(x * 100).round()}%';

  static String teksBertanda(int n, String satuan) =>
      '${n > 0 ? '+' : ''}$n $satuan';

  static String teksPan(double p) {
    final n = (p * 100).round();
    if (n == 0) return 'C';
    return n < 0 ? 'L${-n}' : 'R$n';
  }

  static String teksHz(double hz) =>
      hz < 1000 ? '${hz.round()} Hz' : '${(hz / 1000).toStringAsFixed(1)} kHz';
}
