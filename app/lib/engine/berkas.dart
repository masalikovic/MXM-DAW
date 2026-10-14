import 'package:flutter/services.dart';

/// Berkas audio yang dipilih pengguna: nama dan isinya.
class BerkasAudio {
  const BerkasAudio({required this.nama, required this.isi});

  final String nama;
  final Uint8List isi;
}

/// Memilih berkas dari HP. Antarmuka ini memisahkan layar dari Android,
/// supaya tes widget bisa memakai pemilih palsu.
abstract interface class PemilihBerkas {
  /// Buka pemilih berkas audio. `null` bila pengguna membatalkan.
  /// Melempar [PemilihGagal] bila berkas tidak bisa dibaca.
  Future<BerkasAudio?> pilihAudio();
}

/// Galat dari pemilih berkas, dengan pesan untuk pengguna.
class PemilihGagal implements Exception {
  const PemilihGagal(this.pesan);

  final String pesan;

  @override
  String toString() => pesan;
}

/// Pemilih sungguhan: Storage Access Framework lewat `MainActivity.kt`.
/// Tanpa izin penyimpanan; Android hanya memberi akses ke berkas yang dipilih.
class PemilihBerkasAndroid implements PemilihBerkas {
  const PemilihBerkasAndroid();

  static const _kanal = MethodChannel('mxmdaw/berkas');

  @override
  Future<BerkasAudio?> pilihAudio() async {
    try {
      final hasil = await _kanal.invokeMapMethod<String, Object>('pilihAudio');
      if (hasil == null) return null;
      return BerkasAudio(
        nama: hasil['nama']! as String,
        isi: hasil['isi']! as Uint8List,
      );
    } on PlatformException catch (e) {
      throw PemilihGagal(e.message ?? e.code);
    }
  }
}
