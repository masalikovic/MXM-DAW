import 'package:flutter_test/flutter_test.dart';
import 'package:mxmdaw/engine/audio.dart';
import 'package:mxmdaw/features/pola/grid.dart';

NoteData not(int tick, int pad, [double velocity = 1]) =>
    NoteData(tick: tick, pad: pad, velocity: velocity, length: 240, pitch: 0);

void main() {
  test('langkah kosong dinyalakan: not 1/16 di awal langkah', () {
    final hasil = GridLangkah.alih(const [], 3, 5);
    expect(hasil.single, not(1200, 3, GridLangkah.velocityBawaan));
  });

  test('langkah berisi dimatikan, termasuk rekaman yang bergeser', () {
    // Rekaman langsung jatuh sedikit setelah awal langkah 2 (tick 480).
    final pola = [not(493, 0), not(480, 1), not(0, 0)];
    final hasil = GridLangkah.alih(pola, 0, 2);
    expect(hasil, [not(480, 1), not(0, 0)], reason: 'pad lain tidak tersentuh');
  });

  test('not dicari per langkah dan per pad', () {
    final pola = [not(239, 0), not(240, 0), not(240, 2)];
    expect(GridLangkah.diLangkah(pola, 0, 0), [not(239, 0)]);
    expect(GridLangkah.diLangkah(pola, 0, 1), [not(240, 0)]);
    expect(GridLangkah.diLangkah(pola, 1, 1), isEmpty);
  });

  test('tahan memutar velocity 1,0 → 0,7 → 0,4 → 1,0', () {
    var pola = [not(0, 0)];
    final urutan = <double>[];
    for (var i = 0; i < 3; i++) {
      pola = GridLangkah.putarVelocity(pola, 0, 0);
      urutan.add(pola.single.velocity);
    }
    expect(urutan, [0.7, 0.4, 1.0]);
    expect(GridLangkah.putarVelocity(pola, 0, 9), same(pola));
  });
}
