import 'package:flutter/services.dart';

import 'bridge/api.dart' as rust;

/// Proyek di disk (ADR-0016): dibuka saat aplikasi mulai, disimpan otomatis.
///
/// Antarmuka ini memisahkan layar dari Rust dan sistem berkas, supaya tes
/// widget bisa memakai proyek palsu.
abstract interface class Proyek {
  /// Buka proyek; mengembalikan peringatan (mis. sampel hilang). Melempar
  /// pesan galat (teks) bila tidak bisa dibuka.
  Future<List<String>> buka();

  /// Simpan model sekarang. Melempar pesan galat (teks) bila gagal.
  Future<void> simpan();

  /// Revisi model; naik setiap ada perubahan.
  int revisi();
}

/// Proyek sungguhan di folder data pribadi aplikasi.
class ProyekRust implements Proyek {
  const ProyekRust(this.folder);

  final String folder;

  static const _kanal = MethodChannel('mxmdaw/berkas');

  /// Folder data pribadi aplikasi di Android; `null` di luar Android.
  static Future<String?> folderAndroid() async {
    try {
      return await _kanal.invokeMethod<String>('folderProyek');
    } on MissingPluginException {
      return null;
    }
  }

  @override
  Future<List<String>> buka() => rust.projectLoad(dir: folder);

  @override
  Future<void> simpan() => rust.projectSave(dir: folder);

  @override
  int revisi() => rust.projectRevision();
}
