import 'dart:async';

import 'proyek.dart';

/// Simpan otomatis (rencana §10): setiap [jeda], bila revisi model berubah
/// sejak simpan terakhir, simpan sekali. Paling banyak satu simpan berjalan.
///
/// Karena mengikuti revisi Rust (bukan tombol di layar), rekaman dari
/// sequencer juga ikut tersimpan.
class SimpanOtomatis {
  SimpanOtomatis(
    this.proyek, {
    this.jeda = const Duration(milliseconds: 500),
    this.onGalat,
    this.onTersimpan,
  });

  final Proyek proyek;
  final Duration jeda;
  final void Function(Object galat)? onGalat;
  final void Function()? onTersimpan;

  Timer? _timer;
  int? _tersimpan;
  Future<void>? _berjalan;

  /// Mulai memantau. Revisi saat ini dianggap sudah tersimpan (baru dibuka).
  void mulai() {
    _tersimpan ??= proyek.revisi();
    _timer ??= Timer.periodic(jeda, (_) => unawaited(_cek()));
  }

  void berhenti() {
    _timer?.cancel();
    _timer = null;
  }

  /// Simpan sekarang bila ada perubahan (mis. aplikasi ke latar). Menunggu
  /// simpan yang sedang berjalan lebih dulu.
  Future<void> simpanSekarang() async {
    await _berjalan;
    await _cek();
  }

  Future<void> _cek() {
    final jalan = _berjalan;
    if (jalan != null) return jalan;
    final revisi = proyek.revisi();
    if (revisi == _tersimpan) return Future.value();
    return _berjalan = _simpan(revisi);
  }

  Future<void> _simpan(int revisi) async {
    try {
      await proyek.simpan();
      // Yang tersimpan adalah revisi saat simpan dimulai; perubahan selama
      // menyimpan akan disimpan pada putaran berikutnya.
      _tersimpan = revisi;
      onTersimpan?.call();
    } on Object catch (galat) {
      onGalat?.call(galat);
    } finally {
      _berjalan = null;
    }
  }
}
