/// Tap tempo (rencana §4.3): ketuk beberapa kali mengikuti lagu, tempo dihitung
/// dari rata-rata jarak antar-ketukan.
///
/// Waktu diberikan pemanggil (bukan dibaca sendiri dari jam), supaya kelas ini
/// bisa dites tanpa menunggu.
class TapTempo {
  /// Jarak lebih lama dari ini dianggap mulai dari awal.
  static const jedaUlang = Duration(seconds: 2);

  /// Jumlah jarak terakhir yang dirata-rata.
  static const jendela = 4;

  final _ketukan = <Duration>[];

  /// Catat satu ketukan pada `waktu`. Mengembalikan BPM (satu desimal,
  /// 40–240) bila sudah ada sedikitnya dua ketukan, atau `null`.
  double? ketuk(Duration waktu) {
    if (_ketukan.isNotEmpty && waktu - _ketukan.last > jedaUlang) {
      _ketukan.clear();
    }
    _ketukan.add(waktu);
    if (_ketukan.length > jendela + 1) _ketukan.removeAt(0);
    if (_ketukan.length < 2) return null;
    final rata =
        (_ketukan.last - _ketukan.first).inMicroseconds / (_ketukan.length - 1);
    final bpm = 60e6 / rata;
    return ((bpm * 10).round() / 10).clamp(40.0, 240.0);
  }
}
