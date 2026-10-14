import 'package:flutter/material.dart';

import 'app/theme.dart';
import 'design_system/tokens.dart';
import 'engine/audio.dart';
import 'engine/engine.dart';
import 'features/beranda.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  final engine = await startEngine();
  runApp(
    MxmDawApp(
      engine: engine,
      audio: engine is EngineReady ? const MesinAudioRust() : null,
    ),
  );
}

/// Akar aplikasi MXM-DAW.
///
/// Tema terang/gelap mengikuti pengaturan HP. [engine] dan [audio] diberikan
/// dari luar, supaya tes widget bisa memakai mesin palsu tanpa Rust (ADR-0005).
class MxmDawApp extends StatelessWidget {
  const MxmDawApp({required this.engine, this.audio, super.key});

  final EngineStatus engine;
  final MesinAudio? audio;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'MXM-DAW',
      debugShowCheckedModeBanner: false,
      theme: mxmTheme(MxmTokens.light, Brightness.light),
      darkTheme: mxmTheme(MxmTokens.dark, Brightness.dark),
      home: Beranda(engine: engine, audio: audio),
    );
  }
}
