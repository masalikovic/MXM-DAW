import 'package:flutter_test/flutter_test.dart';
import 'package:mxmdaw/features/keys/keys.dart';

void main() {
  test('kiri bawah nada terendah, naik ke kanan lalu ke atas', () {
    expect(Keys.derajat(12), 0, reason: 'pad 13 (kiri bawah)');
    expect(Keys.derajat(15), 3);
    expect(Keys.derajat(8), 4);
    expect(Keys.derajat(3), 15, reason: 'pad 4 (kanan atas)');
  });

  test('skala mayor melompati nada di luar skala, lalu naik oktaf', () {
    final nada = [
      for (final p in [12, 13, 14, 15, 8, 9, 10, 11])
        Keys.nada(p, Skala.mayor, 0),
    ];
    expect(nada, [0, 2, 4, 5, 7, 9, 11, 12]);
  });

  test('pentatonik dan oktaf', () {
    // Pentatonik punya 5 nada per oktaf: derajat 4 = 10, derajat 5 = 12.
    expect(Keys.nada(8, Skala.pentatonik, 0), 10);
    expect(Keys.nada(9, Skala.pentatonik, 0), 12);
    expect(Keys.nada(12, Skala.kromatik, -1), -12);
    expect(Keys.nada(3, Skala.kromatik, 2), 15 + 24);
  });

  test('nama nada', () {
    expect(Keys.namaNada(0), 'C3');
    expect(Keys.namaNada(7), 'G3');
    expect(Keys.namaNada(-1), 'B2');
    expect(Keys.namaNada(13), 'C#4');
  });

  test('skala berputar', () {
    expect(Skala.pentatonik.berikut, Skala.kromatik);
  });
}
