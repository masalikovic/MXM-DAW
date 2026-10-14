import 'package:flutter/material.dart';

import 'app/theme.dart';
import 'design_system/tokens.dart';
import 'engine/engine.dart';

Future<void> main() async {
  final engine = await startEngine();
  runApp(MxmDawApp(engine: engine));
}

/// Akar aplikasi MXM-DAW.
///
/// Tema terang/gelap mengikuti pengaturan HP. [engine] diberikan dari luar,
/// supaya tes widget bisa memakai status palsu tanpa Rust (ADR-0005).
class MxmDawApp extends StatelessWidget {
  const MxmDawApp({required this.engine, super.key});

  final EngineStatus engine;

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'MXM-DAW',
      debugShowCheckedModeBanner: false,
      theme: mxmTheme(MxmTokens.light, Brightness.light),
      darkTheme: mxmTheme(MxmTokens.dark, Brightness.dark),
      home: _Beranda(engine: engine),
    );
  }
}

class _Beranda extends StatelessWidget {
  const _Beranda({required this.engine});

  final EngineStatus engine;

  @override
  Widget build(BuildContext context) {
    final t = MxmTokens.of(context);
    final keterangan = TextStyle(color: t.textMuted, fontSize: 13);
    return Scaffold(
      body: Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisSize: MainAxisSize.min,
            children: [
              Text(
                'MXM-DAW',
                style: TextStyle(
                  color: t.legendOnLight,
                  fontSize: 28,
                  fontWeight: FontWeight.w600,
                  letterSpacing: 2,
                ),
              ),
              const SizedBox(height: 8),
              ...switch (engine) {
                EngineReady(:final version) => [
                  Text('MESIN $version', style: keterangan),
                ],
                EngineUnavailable(:final reason) => [
                  Text('MESIN TIDAK TERMUAT', style: keterangan),
                  const SizedBox(height: 4),
                  Text(
                    reason,
                    style: keterangan.copyWith(fontSize: 11),
                    textAlign: TextAlign.center,
                  ),
                ],
              },
            ],
          ),
        ),
      ),
    );
  }
}
