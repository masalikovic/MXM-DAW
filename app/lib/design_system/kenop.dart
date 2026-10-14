import 'dart:math' as math;

import 'package:flutter/gestures.dart';
import 'package:flutter/material.dart';

import 'tokens.dart';

/// Kenop (rencana §5.4): Ø 56 dp, geser vertikal 200 dp = penuh, ketuk ganda
/// = nilai bawaan, cincin nilai di sekelilingnya.
///
/// Nilai selalu 0..1. Arti nilai (detik, semitone, Hz) diatur pemanggil lewat
/// [teks], supaya satu komponen dipakai untuk semua parameter.
class Kenop extends StatefulWidget {
  const Kenop({
    required this.label,
    required this.nilai,
    required this.teks,
    required this.onUbah,
    this.bawaan = 0,
    this.aktif = true,
    super.key,
  });

  final String label;

  /// 0..1.
  final double nilai;

  /// Nilai yang dibaca manusia, mis. "120 ms".
  final String teks;
  final ValueChanged<double> onUbah;

  /// Nilai saat diketuk ganda.
  final double bawaan;
  final bool aktif;

  static const diameter = 56.0;

  /// Jarak geser vertikal untuk 0 → 1.
  static const jarakPenuh = 200.0;

  @override
  State<Kenop> createState() => _KenopState();
}

class _KenopState extends State<Kenop> {
  /// Nilai selama digeser. Beberapa event geser bisa tiba dalam satu frame,
  /// sebelum [Kenop.nilai] diperbarui pemanggil; tanpa ini sebagian gerakan
  /// hilang.
  double? _geser;

  void _ubah(double dy) {
    final baru = ((_geser ?? widget.nilai) - dy / Kenop.jarakPenuh).clamp(
      0.0,
      1.0,
    );
    _geser = baru;
    widget.onUbah(baru);
  }

  @override
  Widget build(BuildContext context) {
    final t = MxmTokens.of(context);
    final Kenop(:label, :teks, :nilai, :aktif, :bawaan, :onUbah) = widget;
    return Semantics(
      slider: true,
      label: label,
      value: teks,
      child: GestureDetector(
        // Hitung dari titik sentuh pertama, supaya 18 dp awal (touch slop)
        // tidak hilang dan 200 dp benar-benar = penuh.
        dragStartBehavior: DragStartBehavior.down,
        onVerticalDragStart: aktif ? (_) => _geser = nilai : null,
        onVerticalDragUpdate: aktif ? (d) => _ubah(d.delta.dy) : null,
        onVerticalDragEnd: (_) => _geser = null,
        onVerticalDragCancel: () => _geser = null,
        onDoubleTap: aktif ? () => onUbah(bawaan) : null,
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            SizedBox.square(
              dimension: Kenop.diameter,
              child: CustomPaint(
                painter: _CincinKenop(
                  nilai: nilai,
                  isi: t.keyDark,
                  // Jalur cincin harus terlihat di atas badan terang maupun gelap.
                  jalur: t.textMuted.withValues(alpha: 0.35),
                  warna: aktif ? t.groupA : t.textMuted,
                ),
                child: Center(
                  child: Text(
                    teks,
                    textAlign: TextAlign.center,
                    style: TextStyle(color: t.legendOnDark, fontSize: 10),
                  ),
                ),
              ),
            ),
            const SizedBox(height: 4),
            Text(
              label,
              style: TextStyle(
                color: t.legendOnLight,
                fontSize: 10,
                fontWeight: FontWeight.w600,
                letterSpacing: 0.5,
              ),
            ),
          ],
        ),
      ),
    );
  }
}

class _CincinKenop extends CustomPainter {
  _CincinKenop({
    required this.nilai,
    required this.isi,
    required this.jalur,
    required this.warna,
  });

  final double nilai;
  final Color isi;
  final Color jalur;
  final Color warna;

  /// Cincin dari jam 7 sampai jam 5 (270°).
  static const _awal = math.pi * 0.75;
  static const _sapuan = math.pi * 1.5;

  @override
  void paint(Canvas canvas, Size size) {
    final pusat = size.center(Offset.zero);
    final r = size.shortestSide / 2;
    canvas.drawCircle(pusat, r - 6, Paint()..color = isi);
    final kotak = Rect.fromCircle(center: pusat, radius: r - 2.5);
    final garis = Paint()
      ..style = PaintingStyle.stroke
      ..strokeWidth = 3
      ..strokeCap = StrokeCap.round;
    canvas
      ..drawArc(kotak, _awal, _sapuan, false, garis..color = jalur)
      ..drawArc(kotak, _awal, _sapuan * nilai, false, garis..color = warna);
  }

  @override
  bool shouldRepaint(_CincinKenop lama) =>
      lama.nilai != nilai || lama.warna != warna || lama.isi != isi;
}
