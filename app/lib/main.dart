import 'package:flutter/material.dart';

import 'app/theme.dart';
import 'design_system/tokens.dart';

void main() {
  runApp(const MxmDawApp());
}

/// Akar aplikasi MXM-DAW.
///
/// Tema terang/gelap mengikuti pengaturan HP. Versi mesin dari Rust masuk di
/// M0 langkah 5.
class MxmDawApp extends StatelessWidget {
  const MxmDawApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'MXM-DAW',
      debugShowCheckedModeBanner: false,
      theme: mxmTheme(MxmTokens.light, Brightness.light),
      darkTheme: mxmTheme(MxmTokens.dark, Brightness.dark),
      home: const _Beranda(),
    );
  }
}

class _Beranda extends StatelessWidget {
  const _Beranda();

  @override
  Widget build(BuildContext context) {
    final t = MxmTokens.of(context);
    return Scaffold(
      body: Center(
        child: Text(
          'MXM-DAW',
          style: TextStyle(
            color: t.legendOnLight,
            fontSize: 28,
            fontWeight: FontWeight.w600,
            letterSpacing: 2,
          ),
        ),
      ),
    );
  }
}
