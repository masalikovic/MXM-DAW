import 'package:flutter/material.dart';

import 'app/theme.dart';
import 'design_system/tokens.dart';
import 'engine/audio.dart';
import 'engine/engine.dart';
import 'engine/proyek.dart';
import 'features/beranda.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final engine = await startEngine();
  // Proyek dibuka sebelum mesin audio mulai (ADR-0016), supaya mesin langsung
  // memainkan isi proyek, bukan kit pabrik lalu berganti.
  Proyek? proyek;
  String? pesan;
  if (engine is EngineReady) {
    final folder = await ProyekRust.folderAndroid();
    if (folder != null) {
      proyek = ProyekRust(folder);
      try {
        final catatan = await proyek.buka();
        if (catatan.isNotEmpty) pesan = 'Proyek dibuka: ${catatan.join('; ')}';
      } on Object catch (galat) {
        pesan = 'Proyek: $galat';
      }
    }
  }
  runApp(
    MxmDawApp(
      engine: engine,
      audio: engine is EngineReady ? const MesinAudioRust() : null,
      proyek: proyek,
      pesanAwal: pesan,
    ),
  );
}

/// Akar aplikasi MXM-DAW.
///
/// Tema terang/gelap mengikuti pengaturan HP. [engine] dan [audio] diberikan
/// dari luar, supaya tes widget bisa memakai mesin palsu tanpa Rust (ADR-0005).
class MxmDawApp extends StatelessWidget {
  const MxmDawApp({
    required this.engine,
    this.audio,
    this.proyek,
    this.pesanAwal,
    super.key,
  });

  final EngineStatus engine;
  final MesinAudio? audio;

  /// Proyek di disk; `null` = tanpa simpan (mis. tes, atau di luar Android).
  final Proyek? proyek;

  /// Pesan saat membuka proyek (peringatan atau galat), ditampilkan sekali.
  final String? pesanAwal;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'MXM-DAW',
      debugShowCheckedModeBanner: false,
      theme: mxmTheme(MxmTokens.light, Brightness.light),
      darkTheme: mxmTheme(MxmTokens.dark, Brightness.dark),
      home: Beranda(
        engine: engine,
        audio: audio,
        proyek: proyek,
        pesanAwal: pesanAwal,
      ),
    );
  }
}
