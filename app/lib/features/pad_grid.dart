import 'dart:async';

import 'package:flutter/material.dart';

import '../design_system/tokens.dart';

/// Kisi 4×4 pad grup A (rencana §6.1). Ukuran pad mengikuti lebar layar,
/// 52–72 dp, dan tidak pernah di bawah 48 dp (rencana §5.4).
class KisiPad extends StatelessWidget {
  const KisiPad({
    required this.aktif,
    required this.nama,
    required this.berbunyi,
    required this.onKetuk,
    required this.onLepas,
    this.onTahan,
    super.key,
  });

  final bool aktif;

  /// Pad ditekan lama: buka halaman Sound (rencana §7).
  final ValueChanged<int>? onTahan;

  /// Nama suara ke-16 pad.
  final List<String> nama;

  /// Bitmask pad yang sedang berbunyi (LED).
  final int berbunyi;
  final void Function(int pad, double velocity) onKetuk;
  final ValueChanged<int> onLepas;

  static const jarak = 8.0;
  static const ukuranMin = 48.0;
  static const ukuranMaks = 72.0;

  /// Ukuran satu pad untuk lebar yang tersedia.
  static double ukuranUntuk(double lebar) =>
      ((lebar - 3 * jarak) / 4).clamp(ukuranMin, ukuranMaks);

  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, batas) {
        final ukuran = ukuranUntuk(batas.maxWidth);
        return Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            for (var baris = 0; baris < 4; baris++)
              Padding(
                padding: EdgeInsets.only(top: baris == 0 ? 0 : jarak),
                child: Row(
                  mainAxisSize: MainAxisSize.min,
                  children: [
                    for (var kolom = 0; kolom < 4; kolom++)
                      Padding(
                        padding: EdgeInsets.only(left: kolom == 0 ? 0 : jarak),
                        child: _pad(baris * 4 + kolom, ukuran),
                      ),
                  ],
                ),
              ),
          ],
        );
      },
    );
  }

  Widget _pad(int i, double ukuran) => Pad(
    key: ValueKey('pad-$i'),
    nomor: i,
    nama: i < nama.length ? nama[i] : '',
    ukuran: ukuran,
    aktif: aktif,
    ledMenyala: berbunyi & (1 << i) != 0,
    onKetuk: (v) => onKetuk(i, v),
    onLepas: () => onLepas(i),
    onTahan: onTahan == null ? null : () => onTahan!(i),
  );
}

/// Satu pad. Memakai `Listener` (event pointer mentah), bukan
/// `GestureDetector`, supaya tidak ada jeda "arena gesture" (rencana §7).
/// Setiap pad melacak jarinya sendiri, jadi beberapa pad bisa ditekan
/// bersamaan. Velocity dari posisi vertikal: atas = 1,0, bawah = 0,4.
class Pad extends StatefulWidget {
  const Pad({
    required this.nomor,
    required this.nama,
    required this.ukuran,
    required this.aktif,
    required this.ledMenyala,
    required this.onKetuk,
    required this.onLepas,
    this.onTahan,
    super.key,
  });

  /// Lama tekan untuk membuka halaman Sound.
  static const lamaTahan = Duration(milliseconds: 500);

  final int nomor;
  final String nama;
  final double ukuran;
  final bool aktif;
  final bool ledMenyala;
  final ValueChanged<double> onKetuk;
  final VoidCallback onLepas;
  final VoidCallback? onTahan;

  @override
  State<Pad> createState() => _PadState();
}

class _PadState extends State<Pad> {
  /// Jari yang sedang menekan pad ini.
  final _jari = <int>{};
  Timer? _tahan;

  @override
  void dispose() {
    _tahan?.cancel();
    super.dispose();
  }

  void _turun(PointerDownEvent e) {
    final velocity = (1.0 - 0.6 * (e.localPosition.dy / widget.ukuran)).clamp(
      0.4,
      1.0,
    );
    widget.onKetuk(velocity);
    setState(() => _jari.add(e.pointer));
    final onTahan = widget.onTahan;
    if (onTahan != null && _jari.length == 1) {
      _tahan = Timer(Pad.lamaTahan, onTahan);
    }
  }

  void _naik(PointerEvent e) {
    if (!_jari.remove(e.pointer)) return;
    // Lepas hanya saat jari terakhir diangkat.
    if (_jari.isEmpty) {
      _tahan?.cancel();
      widget.onLepas();
    }
    setState(() {});
  }

  @override
  Widget build(BuildContext context) {
    final t = MxmTokens.of(context);
    final ditekan = _jari.isNotEmpty;
    return Semantics(
      button: true,
      enabled: widget.aktif,
      label: 'Pad ${widget.nomor + 1} ${widget.nama}',
      child: Listener(
        onPointerDown: widget.aktif ? _turun : null,
        onPointerUp: _naik,
        onPointerCancel: _naik,
        child: AnimatedContainer(
          duration: Duration(milliseconds: ditekan ? 60 : 140),
          width: widget.ukuran,
          height: widget.ukuran,
          transform: Matrix4.translationValues(0, ditekan ? 2 : 0, 0),
          decoration: BoxDecoration(
            color: widget.aktif ? t.keyDark : t.bodyEdge,
            borderRadius: BorderRadius.circular(6),
            boxShadow: [
              BoxShadow(
                color: Colors.black.withValues(alpha: 0.35),
                offset: Offset(0, ditekan ? 0.5 : 2.5),
                blurRadius: ditekan ? 1 : 3,
              ),
            ],
          ),
          child: Stack(
            children: [
              Positioned(
                top: 6,
                left: 6,
                child: RepaintBoundary(
                  child: AnimatedContainer(
                    key: ValueKey('led-${widget.nomor}'),
                    duration: Duration(
                      milliseconds: widget.ledMenyala ? 80 : 0,
                    ),
                    width: 6,
                    height: 6,
                    decoration: BoxDecoration(
                      shape: BoxShape.circle,
                      color: widget.ledMenyala
                          ? t.groupA
                          : t.keyLight.withValues(alpha: 0.15),
                    ),
                  ),
                ),
              ),
              Positioned(
                top: 4,
                right: 6,
                child: Text(
                  '${widget.nomor + 1}',
                  style: TextStyle(
                    color: t.legendOnDark.withValues(alpha: 0.7),
                    fontSize: 10,
                  ),
                ),
              ),
              Align(
                alignment: Alignment.bottomCenter,
                child: Padding(
                  padding: const EdgeInsets.only(bottom: 6),
                  child: Text(
                    widget.nama,
                    maxLines: 1,
                    overflow: TextOverflow.clip,
                    style: TextStyle(
                      color: t.legendOnDark,
                      fontSize: 10,
                      fontWeight: FontWeight.w600,
                      letterSpacing: 0.5,
                    ),
                  ),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
