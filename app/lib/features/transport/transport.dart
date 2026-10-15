import 'package:flutter/material.dart';

import '../../design_system/tokens.dart';
import '../../engine/audio.dart';

/// Layar segmen (rencana §5.4, §6.1): posisi BAR.BEAT, tempo, panjang pola,
/// dan status rekam. Saat count-in, bar ditampilkan negatif (−1.1 … −1.4).
class LayarSegmen extends StatelessWidget {
  const LayarSegmen({
    required this.status,
    required this.seq,
    this.grup = 0,
    super.key,
  });

  /// Grup yang ditampilkan (0..3); posisi dan jumlah not mengikuti polanya.
  final int grup;
  final TransportState status;
  final SeqConfig seq;

  @override
  Widget build(BuildContext context) {
    final t = MxmTokens.of(context);
    final posisi = status.playing
        ? '${status.bar < 0 ? '' : ' '}${status.bar}.${status.beat}'
        : ' 1.1';
    final gaya = TextStyle(
      color: t.displayOn,
      fontFamily: 'monospace',
      fontSize: 13,
    );
    return Container(
      key: const ValueKey('layar-segmen'),
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: t.displayBg,
        borderRadius: BorderRadius.circular(6),
      ),
      child: Row(
        children: [
          Text(
            posisi,
            style: gaya.copyWith(fontSize: 26, fontWeight: FontWeight.w600),
          ),
          const SizedBox(width: 16),
          // Setiap baris tetap satu baris (mengecil bila perlu), supaya tinggi
          // layar segmen tidak bertambah di HP sempit.
          Expanded(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                _satuBaris(
                  Text(
                    'BPM ${seq.bpm.toStringAsFixed(1).padLeft(5, '0')}'
                    '   SW ${(seq.swing * 100).round()}%',
                    style: gaya,
                  ),
                ),
                _satuBaris(
                  Row(
                    children: [
                      Text(
                        'POLA ${'ABCD'[grup.clamp(0, 3)]} ${seq.bars} BAR'
                        '   NOT ${status.noteCount}',
                        style: gaya,
                      ),
                      if (status.recording)
                        Text(
                          '   ● REC',
                          style: gaya.copyWith(
                            color: t.displayAccent,
                            fontWeight: FontWeight.w600,
                          ),
                        )
                      else if (status.playing)
                        Text(
                          '   \u25B6\uFE0E',
                          style: gaya.copyWith(color: t.displayAccent),
                        ),
                    ],
                  ),
                ),
              ],
            ),
          ),
        ],
      ),
    );
  }
}

Widget _satuBaris(Widget isi) => FittedBox(
  fit: BoxFit.scaleDown,
  alignment: Alignment.centerLeft,
  child: isi,
);

/// Tombol transport (rencana §6.1): dua baris di bawah pad.
///
/// Simbol ▶ ditulis `\u25B6\uFE0E`: tanpa penanda U+FE0E, Android
/// menggambarnya sebagai emoji berwarna.
class PanelTransport extends StatelessWidget {
  const PanelTransport({
    required this.aktif,
    required this.status,
    required this.seq,
    required this.onMain,
    required this.onRekam,
    required this.onSeq,
    required this.onTap,
    required this.onHapus,
    required this.onGrid,
    super.key,
  });

  final bool aktif;
  final TransportState status;
  final SeqConfig seq;
  final VoidCallback onMain;
  final VoidCallback onRekam;

  /// Perubahan pengaturan, sebagai fungsi dari nilai **terbaru**. Dua ketukan
  /// cepat sebelum layar digambar ulang tidak boleh sama-sama memakai nilai
  /// lama (pelajaran yang sama dengan kenop di M2).
  final ValueChanged<SeqConfig Function(SeqConfig)> onSeq;
  final VoidCallback onTap;
  final VoidCallback onHapus;

  /// Buka halaman Pola (grid langkah, swing, kuantisasi).
  final VoidCallback onGrid;

  /// Pilihan panjang pola.
  static const pilihanBar = [1, 2, 4, 8, 16];

  static int _barBerikut(int bars) =>
      pilihanBar[(pilihanBar.indexOf(bars) + 1) % pilihanBar.length];

  @override
  Widget build(BuildContext context) {
    final t = MxmTokens.of(context);
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        Row(
          children: [
            _Tombol(
              kunci: 'bar',
              label: 'BAR ${seq.bars}',
              onTekan: aktif
                  ? () => onSeq((s) => s.salin(bars: _barBerikut(s.bars)))
                  : null,
            ),
            _Tombol(
              kunci: 'bpm-kurang',
              label: '−',
              onTekan: aktif
                  ? () => onSeq((s) => s.salin(bpm: s.bpm - 1))
                  : null,
            ),
            _Tombol(kunci: 'tap', label: 'TAP', onTekan: aktif ? onTap : null),
            _Tombol(
              kunci: 'bpm-tambah',
              label: '+',
              onTekan: aktif
                  ? () => onSeq((s) => s.salin(bpm: s.bpm + 1))
                  : null,
            ),
            _Tombol(
              kunci: 'metronom',
              label: 'METRO',
              nyala: seq.metronome,
              onTekan: aktif
                  ? () => onSeq((s) => s.salin(metronome: !s.metronome))
                  : null,
            ),
          ],
        ),
        const SizedBox(height: 8),
        Row(
          children: [
            _Tombol(
              kunci: 'hapus',
              label: 'HAPUS',
              onTahan: aktif ? onHapus : null,
            ),
            _Tombol(
              kunci: 'count-in',
              label: 'IN ${seq.countInBars}',
              onTekan: aktif
                  ? () => onSeq(
                      (s) => s.salin(countInBars: (s.countInBars + 1) % 3),
                    )
                  : null,
            ),
            _Tombol(
              kunci: 'grid',
              label: 'GRID',
              onTekan: aktif ? onGrid : null,
            ),
            _Tombol(
              kunci: 'rekam',
              label: '● REC',
              lebar: 2,
              nyala: status.recording,
              warnaNyala: t.displayAccent,
              onTekan: aktif ? onRekam : null,
            ),
            _Tombol(
              kunci: 'main',
              label: status.playing ? '■ STOP' : '\u25B6\uFE0E PLAY',
              lebar: 2,
              nyala: status.playing,
              onTekan: aktif ? onMain : null,
            ),
          ],
        ),
      ],
    );
  }
}

class _Tombol extends StatelessWidget {
  const _Tombol({
    required this.kunci,
    required this.label,
    this.onTekan,
    this.onTahan,
    this.nyala = false,
    this.warnaNyala,
    this.lebar = 1,
  });

  final String kunci;
  final String label;
  final VoidCallback? onTekan;
  final VoidCallback? onTahan;
  final bool nyala;
  final Color? warnaNyala;
  final int lebar;

  @override
  Widget build(BuildContext context) {
    final t = MxmTokens.of(context);
    final aktif = onTekan != null || onTahan != null;
    return Expanded(
      flex: lebar,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 3),
        child: Material(
          color: aktif ? t.keyDark : t.bodyEdge,
          borderRadius: BorderRadius.circular(6),
          child: InkWell(
            key: ValueKey(kunci),
            borderRadius: BorderRadius.circular(6),
            onTap: onTekan,
            onLongPress: onTahan,
            child: SizedBox(
              height: 44,
              child: Center(
                child: Text(
                  label,
                  style: TextStyle(
                    color: nyala ? (warnaNyala ?? t.groupA) : t.legendOnDark,
                    fontSize: 12,
                    fontWeight: FontWeight.w600,
                    letterSpacing: 0.5,
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
  }
}
