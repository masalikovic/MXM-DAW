import 'package:flutter/material.dart';

void main() {
  runApp(const MxmDawApp());
}

/// Akar aplikasi MXM-DAW.
///
/// Di M0 langkah 3 isinya sengaja minimal: hanya membuktikan bahwa aplikasi
/// Flutter bisa dibangun dan dites. Tema MXM (token warna) masuk di langkah 4,
/// dan versi mesin dari Rust di langkah 5.
class MxmDawApp extends StatelessWidget {
  const MxmDawApp({super.key});

  @override
  Widget build(BuildContext context) {
    return const MaterialApp(
      title: 'MXM-DAW',
      debugShowCheckedModeBanner: false,
      home: Scaffold(body: Center(child: Text('MXM-DAW'))),
    );
  }
}
