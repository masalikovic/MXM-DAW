import 'dart:async';

import 'package:flutter/material.dart';

import '../../design_system/kenop.dart';
import '../../design_system/tokens.dart';
import '../../engine/audio.dart';
import '../../engine/papan_klip.dart';
import '../suara/pemetaan.dart';
import 'grid.dart';

/// Halaman Pola (rencana §4.3, §6.2): grid 16 langkah per bar untuk satu pad,
/// swing, dan kuantisasi. Not hasil rekaman langsung juga tampil di sini.
class HalamanPola extends StatefulWidget {
  const HalamanPola({
    required this.audio,
    required this.nama,
    this.grup = 0,
    this.pad = 0,
    this.klip,
    super.key,
  });

  final MesinAudio audio;

  /// Grup yang polanya disunting (0..3).
  final int grup;

  /// Nama suara 16 pad grup ini.
  final List<String> nama;

  /// Pad (lokal 0..15) yang dipilih saat halaman dibuka.
  final int pad;

  /// Papan klip bersama layar lain; `null` = papan klip sendiri.
  final PapanKlip? klip;

  @override
  State<HalamanPola> createState() => _HalamanPolaState();
}

class _HalamanPolaState extends State<HalamanPola> {
  late int _pad = widget.pad;
  late List<String> _nama = widget.nama;
  late final PapanKlip _klip = widget.klip ?? PapanKlip();
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
    _seq = widget.audio.pengaturanSeq(widget.grup);
    _not = widget.audio.notPola(widget.grup);
    _status = widget.audio.statusTransport(widget.grup);
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
    final status = widget.audio.statusTransport(widget.grup);
    if (status == _status) return;
    setState(() {
      if (status.noteCount != _status.noteCount) {
        _not = widget.audio.notPola(widget.grup);
      }
      _status = status;
    });
  }

  void _aturNot(List<NoteData> baru) {
    if (widget.audio.aturNotPola(widget.grup, baru)) {
      setState(() => _not = widget.audio.notPola(widget.grup));
    }
  }

  String get _hurufGrup => 'ABCD'[widget.grup.clamp(0, 3)];

  void _menu(String pilihan) {
    final audio = widget.audio;
    final grup = widget.grup;
    final String pesan;
    switch (pilihan) {
      case 'salin-pola':
        _klip.pola = (not: List.of(_not), bars: _seq.bars);
        pesan = 'Pola $_hurufGrup disalin';
      case 'tempel-pola':
        final pola = _klip.pola;
        if (pola == null) return;
        // Panjang dulu, supaya not di bar akhir tidak dibuang.
        _aturSeq((s) => s.salin(bars: pola.bars));
        _aturNot(pola.not);
        pesan = 'Pola ditempel ke $_hurufGrup';
      case 'salin-grup':
        _klip.grup = grup;
        pesan = 'Grup $_hurufGrup disalin';
      case 'tempel-grup':
        final dari = _klip.grup;
        if (dari == null || !audio.salinGrup(dari, grup)) return;
        setState(() {
          _seq = audio.pengaturanSeq(grup);
          _not = audio.notPola(grup);
          _nama = audio.namaPad().sublist(grup * 16, grup * 16 + 16);
        });
        pesan = 'Grup ${'ABCD'[dari]} ditempel ke $_hurufGrup';
      default:
        return;
    }
    ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text(pesan)));
  }

  void _aturSeq(SeqConfig Function(SeqConfig) ubah) {
    setState(
      () => _seq = widget.audio.aturPengaturanSeq(widget.grup, ubah(_seq)),
    );
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
          '$_hurufGrup · ${_seq.bars} BAR  ·  '
          'PAD ${_pad + 1} '
          '${_pad < _nama.length ? _nama[_pad] : ''}',
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
          PopupMenuButton<String>(
            key: const ValueKey('menu-pola'),
            tooltip: 'Salin/tempel',
            onSelected: _menu,
            itemBuilder: (_) => [
              const PopupMenuItem(
                value: 'salin-pola',
                child: Text('SALIN POLA'),
              ),
              PopupMenuItem(
                value: 'tempel-pola',
                enabled: _klip.pola != null,
                child: const Text('TEMPEL POLA'),
              ),
              const PopupMenuItem(
                value: 'salin-grup',
                child: Text('SALIN GRUP (SUARA + POLA)'),
              ),
              PopupMenuItem(
                value: 'tempel-grup',
                enabled: _klip.grup != null && _klip.grup != widget.grup,
                child: const Text('TEMPEL GRUP'),
              ),
            ],
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
                  for (var i = 0; i < _nama.length; i++)
                    Padding(
                      padding: const EdgeInsets.only(right: 6),
                      child: ChoiceChip(
                        key: ValueKey('pilih-pad-$i'),
                        label: Text('${i + 1} ${_nama[i]}'),
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
