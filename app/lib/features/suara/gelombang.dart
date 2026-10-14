import 'dart:typed_data';

import 'package:flutter/material.dart';

import '../../design_system/tokens.dart';

/// Gambar gelombang sampel dengan penanda start/end yang bisa digeser
/// (rencana §4.2, §5.4). Ringkasan gelombang dihitung sekali oleh Rust;
/// menggeser penanda hanya menggambar ulang widget ini.
class Gelombang extends StatefulWidget {
  const Gelombang({
    required this.puncak,
    required this.frames,
    required this.start,
    required this.end,
    required this.onUbah,
    super.key,
  });

  /// Jumlah pasangan (min, max) yang diminta dari Rust.
  static const titik = 300;
  static const tinggi = 120.0;

  /// `[min0, max0, min1, max1, …]`.
  final Float32List puncak;
  final int frames;
  final int start;
  final int end;
  final void Function(int start, int end) onUbah;

  @override
  State<Gelombang> createState() => _GelombangState();
}

class _GelombangState extends State<Gelombang> {
  /// Penanda yang sedang digeser: true = start, false = end.
  bool? _geserStart;

  int _frameDi(double x, double lebar) =>
      (x / lebar * widget.frames).round().clamp(0, widget.frames);

  void _mulai(DragStartDetails d, double lebar) {
    final f = _frameDi(d.localPosition.dx, lebar);
    final akhir = widget.end.clamp(0, widget.frames);
    _geserStart = (f - widget.start).abs() <= (f - akhir).abs();
  }

  void _geser(DragUpdateDetails d, double lebar) {
    final f = _frameDi(d.localPosition.dx, lebar);
    final akhir = widget.end.clamp(0, widget.frames);
    if (_geserStart ?? true) {
      widget.onUbah(f.clamp(0, akhir - 2), akhir);
    } else {
      widget.onUbah(widget.start, f.clamp(widget.start + 2, widget.frames));
    }
  }

  @override
  Widget build(BuildContext context) {
    final t = MxmTokens.of(context);
    return LayoutBuilder(
      builder: (context, batas) {
        final lebar = batas.maxWidth;
        return GestureDetector(
          key: const ValueKey('gelombang'),
          onHorizontalDragStart: widget.frames > 2
              ? (d) => _mulai(d, lebar)
              : null,
          onHorizontalDragUpdate: widget.frames > 2
              ? (d) => _geser(d, lebar)
              : null,
          child: RepaintBoundary(
            child: CustomPaint(
              size: Size(lebar, Gelombang.tinggi),
              painter: _LukisGelombang(
                puncak: widget.puncak,
                awal: widget.frames == 0 ? 0 : widget.start / widget.frames,
                akhir: widget.frames == 0
                    ? 1
                    : widget.end.clamp(0, widget.frames) / widget.frames,
                latar: t.displayBg,
                garis: t.displayOn,
                redup: t.displayOff,
                penanda: t.displayAccent,
              ),
            ),
          ),
        );
      },
    );
  }
}

class _LukisGelombang extends CustomPainter {
  _LukisGelombang({
    required this.puncak,
    required this.awal,
    required this.akhir,
    required this.latar,
    required this.garis,
    required this.redup,
    required this.penanda,
  });

  final Float32List puncak;
  final double awal;
  final double akhir;
  final Color latar;
  final Color garis;
  final Color redup;
  final Color penanda;

  @override
  void paint(Canvas canvas, Size size) {
    final r = RRect.fromRectAndRadius(
      Offset.zero & size,
      const Radius.circular(6),
    );
    canvas
      ..save()
      ..clipRRect(r)
      ..drawRect(Offset.zero & size, Paint()..color = latar);
    final n = puncak.length ~/ 2;
    final tengah = size.height / 2;
    final kuas = Paint()..strokeWidth = 1;
    for (var i = 0; i < n; i++) {
      final x = (i + 0.5) / n * size.width;
      final posisi = (i + 0.5) / n;
      kuas.color = posisi >= awal && posisi <= akhir ? garis : redup;
      canvas.drawLine(
        Offset(x, tengah - puncak[2 * i + 1] * tengah * 0.95),
        Offset(x, tengah - puncak[2 * i] * tengah * 0.95),
        kuas,
      );
    }
    final p = Paint()
      ..color = penanda
      ..strokeWidth = 2;
    for (final f in [awal, akhir]) {
      final x = f * size.width;
      canvas.drawLine(Offset(x, 0), Offset(x, size.height), p);
    }
    canvas.restore();
  }

  @override
  bool shouldRepaint(_LukisGelombang lama) =>
      lama.awal != awal ||
      lama.akhir != akhir ||
      !identical(lama.puncak, puncak) ||
      lama.garis != garis;
}
