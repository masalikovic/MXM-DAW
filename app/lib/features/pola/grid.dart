import '../../engine/audio.dart';

/// Logika grid langkah (rencana §4.3): fungsi murni atas daftar not, jadi
/// mudah dites. Layar hanya memanggilnya lalu mengirim hasilnya ke Rust.
abstract final class GridLangkah {
  /// Tick per ketukan dan per langkah 1/16 (sama dengan `daw_core::seq`).
  static const ppqn = 960;
  static const tickLangkah = ppqn ~/ 4;
  static const langkahPerBar = 16;

  /// Velocity baru saat langkah dinyalakan.
  static const velocityBawaan = 0.8;

  /// Urutan velocity saat langkah ditekan lama.
  static const tingkatVelocity = [1.0, 0.7, 0.4];

  /// Not `pad` yang dimulai di dalam langkah `langkah` (dari awal pola).
  static List<NoteData> diLangkah(List<NoteData> not, int pad, int langkah) {
    final awal = langkah * tickLangkah;
    return [
      for (final n in not)
        if (n.pad == pad && n.tick >= awal && n.tick < awal + tickLangkah) n,
    ];
  }

  /// Nyalakan langkah (not baru, panjang 1/16) atau matikan (hapus semua not
  /// `pad` di langkah itu, termasuk hasil rekaman yang sedikit bergeser).
  static List<NoteData> alih(List<NoteData> not, int pad, int langkah) {
    final ada = diLangkah(not, pad, langkah);
    if (ada.isNotEmpty) {
      return [
        for (final n in not)
          if (!ada.contains(n)) n,
      ];
    }
    return [
      ...not,
      NoteData(
        tick: langkah * tickLangkah,
        pad: pad,
        velocity: velocityBawaan,
        length: tickLangkah,
      ),
    ];
  }

  /// Velocity berikutnya (1,0 → 0,7 → 0,4 → 1,0) untuk not di langkah itu.
  static List<NoteData> putarVelocity(
    List<NoteData> not,
    int pad,
    int langkah,
  ) {
    final ada = diLangkah(not, pad, langkah);
    if (ada.isEmpty) return not;
    final sekarang = ada.first.velocity;
    final i = tingkatVelocity.indexWhere((v) => (v - sekarang).abs() < 0.05);
    final berikut = tingkatVelocity[(i + 1) % tingkatVelocity.length];
    return [
      for (final n in not)
        if (ada.contains(n))
          NoteData(
            tick: n.tick,
            pad: n.pad,
            velocity: berikut,
            length: n.length,
          )
        else
          n,
    ];
  }
}
