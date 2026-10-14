import 'dart:async';

import 'package:flutter/material.dart';

import '../../design_system/kenop.dart';
import '../../design_system/tokens.dart';
import '../../engine/audio.dart';
import '../suara/pemetaan.dart';
import 'grid.dart';

/// Halaman Pola (rencana §4.3, §6.2): grid 16 langkah per bar untuk satu pad,
/// swing, dan kuantisasi. Not hasil rekaman langsung juga tampil di sini.
class HalamanPola extends StatefulWidget {
  const HalamanPola({
    required this.audio,
    required this.nama,
    this.pad = 0,
    super.key,
  });

  final MesinAudio audio;

  /// Nama suara ke-16 pad.
  final List<String> nama;

  /// Pad yang dipilih saat halaman dibuka.
  final int pad;

  @override
  State<HalamanPola> createState() => _HalamanPolaState();
}

class _HalamanPolaState extends State<HalamanPola> {
  late int _pad = widget.pad;
  int _halaman = 0;
  late SeqConfig _seq;
  late List<NoteData> _not;
  late TransportState _status;
  Timer? _penyegar;

  /// Pilihan grid kuantisasi: label → tick.
  static const _grid = {
    'MATI': 0,
    '1/8': 480,
    '1/16': 240,
    '1/16T': 160,
    '1/32': 120,
  };

  @override
  void initState() {
    super.initState();
    _seq = widget.audio.pengaturanSeq();
    _not = widget.audio.notPola();
    _status = widget.audio.statusTransport();
    _penyegar = Timer.periodic(
      const Duration(milliseconds: 33),
      (_) => _segarkan(),
    );
  }

  @override
  void dispose() {
    _penyegar?.cancel();
    super.dispose();
  }

  /// Playhead dan rekaman yang masuk saat halaman terbuka.
  void _segarkan() {
    if (!mounted) return;
    final status = widget.audio.statusTransport();
    if (status == _status) return;
    setState(() {
      if (status.noteCount != _status.noteCount) {
        _not = widget.audio.notPola();
      }
      _status = status;
    });
  }

  void _aturNot(List<NoteData> baru) {
    if (widget.audio.aturNotPola(baru)) {
      setState(() => _not = widget.audio.notPola());
    }
  }

  void _aturSeq(SeqConfig Function(SeqConfig) ubah) {
    setState(() => _seq = widget.audio.aturPengaturanSeq(ubah(_seq)));
  }

  @override
  Widget build(BuildContext context) {
    final t = MxmTokens.of(context);
    final halaman = _halaman.clamp(0, _seq.bars - 1);
    final judul = TextStyle(
      color: t.legendOnLight,
      fontSize: 12,
      fontWeight: FontWeight.w600,
      letterSpacing: 1,
    );
    return Scaffold(
      appBar: AppBar(
        title: Text(
          '${_seq.bars} BAR  ·  PAD ${_pad + 1} '
          '${_pad < widget.nama.length ? widget.nama[_pad] : ''}',
        ),
        actions: [
          IconButton(
            key: const ValueKey('pola-main'),
            tooltip: _status.playing ? 'Berhenti' : 'Mainkan',
            icon: Icon(_status.playing ? Icons.stop : Icons.play_arrow),
            onPressed: () => _status.playing
                ? widget.audio.hentikan()
                : widget.audio.mainkan(rekam: false),
          ),
        ],
      ),
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            // Satu baris yang bisa digeser, supaya grid langsung terlihat.
            SingleChildScrollView(
              scrollDirection: Axis.horizontal,
              child: Row(
                children: [
                  for (var i = 0; i < widget.nama.length; i++)
                    Padding(
                      padding: const EdgeInsets.only(right: 6),
                      child: ChoiceChip(
                        key: ValueKey('pilih-pad-$i'),
                        label: Text('${i + 1} ${widget.nama[i]}'),
                        selected: i == _pad,
                        onSelected: (_) => setState(() => _pad = i),
                      ),
                    ),
                ],
              ),
            ),
            if (_seq.bars > 1) ...[
              const SizedBox(height: 8),
              Wrap(
                spacing: 6,
                runSpacing: 6,
                children: [
                  for (var b = 0; b < _seq.bars; b++)
                    ChoiceChip(
                      key: ValueKey('halaman-$b'),
                      label: Text('${b + 1}'),
                      selected: b == halaman,
                      onSelected: (_) => setState(() => _halaman = b),
                    ),
                ],
              ),
            ],
            const SizedBox(height: 16),
            Text(
              'LANGKAH  ·  ketuk = nyala/mati, tahan = velocity',
              style: judul,
            ),
            const SizedBox(height: 8),
            _Kisi(
              not: _not,
              pad: _pad,
              halaman: halaman,
              playhead: _status.playing ? _status.step : -1,
              onKetuk: (l) => _aturNot(GridLangkah.alih(_not, _pad, l)),
              onTahan: (l) =>
                  _aturNot(GridLangkah.putarVelocity(_not, _pad, l)),
            ),
            const SizedBox(height: 24),
            Text('TIMING', style: judul),
            const SizedBox(height: 8),
            Wrap(
              spacing: 6,
              children: [
                for (final MapEntry(key: label, value: tick) in _grid.entries)
                  ChoiceChip(
                    key: ValueKey('grid-$label'),
                    label: Text(label),
                    selected: tick == _seq.quantizeGrid,
                    onSelected: (_) =>
                        _aturSeq((s) => s.salin(quantizeGrid: tick)),
                  ),
              ],
            ),
            const SizedBox(height: 12),
            Wrap(
              spacing: 16,
              children: [
                Kenop(
                  label: 'KUANTISASI',
                  nilai: _seq.quantizeStrength,
                  aktif: _seq.quantizeGrid != 0,
                  teks: Pemetaan.teksPersen(_seq.quantizeStrength),
                  onUbah: (x) => _aturSeq((s) => s.salin(quantizeStrength: x)),
                ),
                Kenop(
                  label: 'SWING',
                  nilai: (_seq.swing - 0.5) / 0.25,
                  teks: Pemetaan.teksPersen(_seq.swing),
                  onUbah: (x) =>
                      _aturSeq((s) => s.salin(swing: 0.5 + x * 0.25)),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

/// 16 sel langkah satu bar, dua baris berisi 8.
class _Kisi extends StatelessWidget {
  const _Kisi({
    required this.not,
    required this.pad,
    required this.halaman,
    required this.playhead,
    required this.onKetuk,
    required this.onTahan,
  });

  final List<NoteData> not;
  final int pad;
  final int halaman;

  /// Langkah yang sedang dimainkan (dari awal pola), atau −1.
  final int playhead;
  final ValueChanged<int> onKetuk;
  final ValueChanged<int> onTahan;

  static const jarak = 4.0;

  @override
  Widget build(BuildContext context) {
    final t = MxmTokens.of(context);
    return LayoutBuilder(
      builder: (context, batas) {
        final ukuran = ((batas.maxWidth - 7 * jarak) / 8).clamp(32.0, 44.0);
        return Column(
          children: [
            for (var baris = 0; baris < 2; baris++)
              Padding(
                padding: EdgeInsets.only(top: baris == 0 ? 0 : jarak),
                child: Row(
                  children: [
                    for (var kolom = 0; kolom < 8; kolom++)
                      Padding(
                        padding: EdgeInsets.only(left: kolom == 0 ? 0 : jarak),
                        child: _sel(t, baris * 8 + kolom, ukuran),
                      ),
                  ],
                ),
              ),
          ],
        );
      },
    );
  }

  Widget _sel(MxmTokens t, int i, double ukuran) {
    final langkah = halaman * GridLangkah.langkahPerBar + i;
    final ada = GridLangkah.diLangkah(not, pad, langkah);
    final velocity = ada.isEmpty ? 0.0 : ada.first.velocity;
    final diPutar = langkah == playhead;
    return GestureDetector(
      key: ValueKey('langkah-$i'),
      onTap: () => onKetuk(langkah),
      onLongPress: ada.isEmpty ? null : () => onTahan(langkah),
      child: Container(
        width: ukuran,
        height: ukuran,
        decoration: BoxDecoration(
          // Awal setiap ketukan sedikit lebih terang, untuk membaca bar.
          color: i % 4 == 0 ? t.keyDark : t.keyDark.withValues(alpha: 0.85),
          borderRadius: BorderRadius.circular(4),
          border: Border.all(
            color: diPutar ? t.displayAccent : Colors.transparent,
            width: 2,
          ),
        ),
        alignment: Alignment.bottomCenter,
        // Tanpa easing: playhead harus terasa mekanis (rencana §5.4).
        child: FractionallySizedBox(
          heightFactor: velocity,
          widthFactor: 1,
          child: velocity == 0
              ? null
              : DecoratedBox(
                  key: ValueKey('isi-$i'),
                  decoration: BoxDecoration(
                    color: t.groupA,
                    borderRadius: BorderRadius.circular(3),
                  ),
                ),
        ),
      ),
    );
  }
}
