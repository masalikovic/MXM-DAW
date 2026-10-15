/// Mode KEYS (rencana §4.1): satu suara dimainkan pada 16 nada sebuah skala.
/// Fungsi murni, jadi mudah dites.
enum Skala {
  kromatik('KROM', [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
  mayor('MAYOR', [0, 2, 4, 5, 7, 9, 11]),
  minor('MINOR', [0, 2, 3, 5, 7, 8, 10]),
  pentatonik('PENTA', [0, 3, 5, 7, 10]);

  const Skala(this.label, this.langkah);

  /// Nama pendek untuk tombol.
  final String label;

  /// Semitone setiap derajat dalam satu oktaf.
  final List<int> langkah;

  Skala get berikut => values[(index + 1) % values.length];
}

abstract final class Keys {
  static const oktafMin = -2;
  static const oktafMaks = 2;

  /// Derajat skala pad lokal 0..15: kiri bawah = 0, naik ke kanan lalu ke
  /// atas, seperti instrumen pad pada umumnya (pad 1 ada di kiri atas).
  static int derajat(int pad) => (3 - pad ~/ 4) * 4 + pad % 4;

  /// Transpose (semitone) pad lokal untuk skala dan oktaf tertentu.
  static int nada(int pad, Skala skala, int oktaf) {
    final d = derajat(pad);
    final n = skala.langkah.length;
    return skala.langkah[d % n] + 12 * (d ~/ n) + 12 * oktaf;
  }

  static const _nama = [
    'C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B', //
  ];

  /// Nama nada bila sampel aslinya dianggap C3, mis. 7 → "G3", −1 → "B2".
  static String namaNada(int semitone) {
    final oktaf = 3 + (semitone / 12).floor();
    return '${_nama[semitone % 12]}$oktaf';
  }
}

/// Pilihan note repeat: label → jarak (tick, 960 per ketukan). 0 = mati.
const pilihanUlang = <String, int>{
  'MATI': 0,
  '1/4': 960,
  '1/8': 480,
  '1/8T': 320,
  '1/16': 240,
  '1/16T': 160,
  '1/32': 120,
};
