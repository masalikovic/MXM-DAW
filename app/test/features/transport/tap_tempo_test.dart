import 'package:flutter_test/flutter_test.dart';
import 'package:mxmdaw/features/transport/tap_tempo.dart';

Duration ms(int n) => Duration(milliseconds: n);

void main() {
  test('dua ketukan berjarak 500 ms = 120 BPM', () {
    final tap = TapTempo();
    expect(tap.ketuk(ms(0)), isNull);
    expect(tap.ketuk(ms(500)), 120);
  });

  test('rata-rata empat jarak terakhir, satu desimal', () {
    final tap = TapTempo();
    for (final t in [0, 600, 1210, 1800, 2410]) {
      tap.ketuk(ms(t));
    }
    // Jendela geser: empat jarak terakhir = (3010 − 600) / 4 = 602,5 ms
    // → 99,585 BPM → 99,6.
    expect(tap.ketuk(ms(3010)), 99.6);
  });

  test('jeda lebih dari 2 detik mulai dari awal', () {
    final tap = TapTempo();
    tap
      ..ketuk(ms(0))
      ..ketuk(ms(500));
    expect(tap.ketuk(ms(5000)), isNull);
    expect(tap.ketuk(ms(5250)), 240);
  });

  test('dibatasi 40–240 BPM', () {
    final tap = TapTempo();
    tap.ketuk(ms(0));
    expect(tap.ketuk(ms(100)), 240);
    final lambat = TapTempo()..ketuk(ms(0));
    expect(lambat.ketuk(ms(1990)), closeTo(40, 0.01)); // 30,2 → 40
  });
}
