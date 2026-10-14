import 'dart:typed_data';

import 'package:flutter/material.dart';

import '../../design_system/kenop.dart';
import '../../design_system/tokens.dart';
import '../../engine/audio.dart';
import '../../engine/berkas.dart';
import 'gelombang.dart';
import 'pemetaan.dart';

/// Halaman Sound (rencana §6.3, §7): menyunting satu pad. Dibuka dengan
/// menekan lama pad.
///
/// Setiap perubahan langsung dikirim ke Rust (sinkron, mikrodetik) dan
/// terdengar pada suara yang sedang berbunyi. Rust membatasi nilainya, lalu
/// layar menampilkan nilai yang **benar-benar** tersimpan (ADR-0002).
class HalamanSuara extends StatefulWidget {
  const HalamanSuara({
    required this.audio,
    required this.pad,
    required this.nama,
    this.pemilih = const PemilihBerkasAndroid(),
    super.key,
  });

  final MesinAudio audio;
  final int pad;
  final String nama;
  final PemilihBerkas pemilih;

  @override
  State<HalamanSuara> createState() => _HalamanSuaraState();
}

class _HalamanSuaraState extends State<HalamanSuara> {
  late PadSettings _p;
  late int _frames;
  late Float32List _puncak;
  late String _nama;
  bool _mengimpor = false;

  @override
  void initState() {
    super.initState();
    _nama = widget.nama;
    _muatSampel();
  }

  void _muatSampel() {
    _p = widget.audio.pengaturan(widget.pad)!;
    _frames = widget.audio.infoSampel(widget.pad).$1;
    _puncak = widget.audio.puncak(widget.pad, Gelombang.titik);
  }

  Future<void> _impor() async {
    final pesan = ScaffoldMessenger.of(context);
    setState(() => _mengimpor = true);
    try {
      final berkas = await widget.pemilih.pilihAudio();
      if (berkas == null) return;
      await widget.audio.imporWav(widget.pad, berkas.nama, berkas.isi);
      if (!mounted) return;
      setState(() {
        _muatSampel();
        _nama = widget.audio.namaPad()[widget.pad];
      });
      pesan.showSnackBar(SnackBar(content: Text('Diimpor: ${berkas.nama}')));
    } on Object catch (galat) {
      pesan.showSnackBar(SnackBar(content: Text('Impor gagal: $galat')));
    } finally {
      if (mounted) setState(() => _mengimpor = false);
    }
  }

  void _ubah(PadSettings baru) {
    final tersimpan = widget.audio.aturPengaturan(widget.pad, baru);
    if (tersimpan != null) setState(() => _p = tersimpan);
  }

  @override
  Widget build(BuildContext context) {
    final t = MxmTokens.of(context);
    final (st, cent) = Pemetaan.pecahPitch(_p.pitch);
    final judul = TextStyle(
      color: t.legendOnLight,
      fontSize: 12,
      fontWeight: FontWeight.w600,
      letterSpacing: 1,
    );
    return Scaffold(
      appBar: AppBar(
        title: Text('A:${(widget.pad + 1).toString().padLeft(2, '0')}  $_nama'),
        actions: [
          TextButton.icon(
            key: const ValueKey('impor'),
            onPressed: _mengimpor ? null : _impor,
            icon: const Icon(Icons.file_open_outlined),
            label: const Text('IMPOR WAV'),
          ),
        ],
      ),
      body: SafeArea(
        child: ListView(
          padding: const EdgeInsets.all(16),
          children: [
            Gelombang(
              key: ValueKey(_puncak),
              puncak: _puncak,
              frames: _frames,
              start: _p.start,
              end: _p.end,
              onUbah: (start, end) => _ubah(_p.salin(start: start, end: end)),
            ),
            const SizedBox(height: 4),
            Text(
              'START ${_p.start}   END ${_p.end}   / $_frames',
              style: TextStyle(
                color: t.textMuted,
                fontSize: 11,
                fontFamily: 'monospace',
              ),
            ),
            const SizedBox(height: 12),
            _TombolDengar(audio: widget.audio, pad: widget.pad),
            const SizedBox(height: 16),
            Text('SUARA', style: judul),
            _baris([
              Kenop(
                label: 'LEVEL',
                nilai: _p.level,
                bawaan: 1,
                teks: Pemetaan.teksPersen(_p.level),
                onUbah: (x) => _ubah(_p.salin(level: x)),
              ),
              Kenop(
                label: 'PAN',
                nilai: Pemetaan.panKeKenop(_p.pan),
                bawaan: 0.5,
                teks: Pemetaan.teksPan(_p.pan),
                onUbah: (x) => _ubah(_p.salin(pan: Pemetaan.pan(x))),
              ),
              Kenop(
                label: 'PITCH',
                nilai: Pemetaan.semitoneKeKenop(st),
                bawaan: 0.5,
                teks: Pemetaan.teksBertanda(st, 'st'),
                onUbah: (x) =>
                    _ubah(_p.salin(pitch: Pemetaan.semitone(x) + cent / 100)),
              ),
              Kenop(
                label: 'CENT',
                nilai: Pemetaan.centKeKenop(cent),
                bawaan: 0.5,
                teks: Pemetaan.teksBertanda(cent, 'c'),
                onUbah: (x) =>
                    _ubah(_p.salin(pitch: st + Pemetaan.cent(x) / 100)),
              ),
            ]),
            const SizedBox(height: 16),
            Text('AMPLOP', style: judul),
            _baris([
              Kenop(
                label: 'ATTACK',
                nilai: Pemetaan.waktuKeKenop(_p.attack),
                teks: Pemetaan.teksWaktu(_p.attack),
                onUbah: (x) => _ubah(_p.salin(attack: Pemetaan.waktu(x))),
              ),
              Kenop(
                label: 'DECAY',
                nilai: Pemetaan.waktuKeKenop(_p.decay),
                teks: Pemetaan.teksWaktu(_p.decay),
                onUbah: (x) => _ubah(_p.salin(decay: Pemetaan.waktu(x))),
              ),
              Kenop(
                label: 'SUSTAIN',
                nilai: _p.sustain,
                bawaan: 1,
                teks: Pemetaan.teksPersen(_p.sustain),
                onUbah: (x) => _ubah(_p.salin(sustain: x)),
              ),
              Kenop(
                label: 'RELEASE',
                nilai: Pemetaan.waktuKeKenop(_p.release),
                bawaan: Pemetaan.waktuKeKenop(0.02),
                teks: Pemetaan.teksWaktu(_p.release),
                onUbah: (x) => _ubah(_p.salin(release: Pemetaan.waktu(x))),
              ),
            ]),
            const SizedBox(height: 16),
            Text('FILTER', style: judul),
            const SizedBox(height: 8),
            _Pilihan<FilterKind>(
              kunci: 'filter',
              nilai: _p.filter,
              pilihan: const {
                FilterKind.off: 'MATI',
                FilterKind.lowPass: 'LP',
                FilterKind.highPass: 'HP',
                FilterKind.bandPass: 'BP',
              },
              onUbah: (f) => _ubah(_p.salin(filter: f)),
            ),
            _baris([
              Kenop(
                label: 'CUTOFF',
                nilai: _p.cutoff,
                bawaan: 1,
                aktif: _p.filter != FilterKind.off,
                teks: Pemetaan.teksHz(Pemetaan.cutoffHz(_p.cutoff)),
                onUbah: (x) => _ubah(_p.salin(cutoff: x)),
              ),
              Kenop(
                label: 'RESO',
                nilai: _p.resonance,
                aktif: _p.filter != FilterKind.off,
                teks: Pemetaan.teksPersen(_p.resonance),
                onUbah: (x) => _ubah(_p.salin(resonance: x)),
              ),
              Kenop(
                label: 'DRIVE',
                nilai: _p.drive,
                teks: Pemetaan.teksPersen(_p.drive),
                onUbah: (x) => _ubah(_p.salin(drive: x)),
              ),
            ]),
            const SizedBox(height: 16),
            Text('MAIN', style: judul),
            const SizedBox(height: 8),
            _Pilihan<PlayKind>(
              kunci: 'mode',
              nilai: _p.play,
              pilihan: const {
                PlayKind.oneShot: 'ONE-SHOT',
                PlayKind.gate: 'GATE',
                PlayKind.loop: 'LOOP',
              },
              onUbah: (m) => _ubah(_p.salin(play: m)),
            ),
            const SizedBox(height: 8),
            Row(
              children: [
                Text('CHOKE', style: judul),
                const Spacer(),
                IconButton(
                  key: const ValueKey('choke-kurang'),
                  onPressed: _p.choke > 0
                      ? () => _ubah(_p.salin(choke: _p.choke - 1))
                      : null,
                  icon: const Icon(Icons.remove),
                ),
                SizedBox(
                  width: 56,
                  child: Text(
                    _p.choke == 0 ? 'TANPA' : 'GRUP ${_p.choke}',
                    textAlign: TextAlign.center,
                    style: TextStyle(color: t.legendOnLight, fontSize: 12),
                  ),
                ),
                IconButton(
                  key: const ValueKey('choke-tambah'),
                  onPressed: _p.choke < 8
                      ? () => _ubah(_p.salin(choke: _p.choke + 1))
                      : null,
                  icon: const Icon(Icons.add),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }

  Widget _baris(List<Widget> kenop) => Padding(
    padding: const EdgeInsets.only(top: 8),
    child: Wrap(spacing: 16, runSpacing: 12, children: kenop),
  );
}

/// Pilihan satu-dari-beberapa (filter, mode main).
class _Pilihan<T> extends StatelessWidget {
  const _Pilihan({
    required this.kunci,
    required this.nilai,
    required this.pilihan,
    required this.onUbah,
    super.key,
  });

  final String kunci;
  final T nilai;
  final Map<T, String> pilihan;
  final ValueChanged<T> onUbah;

  @override
  Widget build(BuildContext context) {
    return Wrap(
      spacing: 8,
      children: [
        for (final MapEntry(:key, :value) in pilihan.entries)
          ChoiceChip(
            key: ValueKey('$kunci-$value'),
            label: Text(value),
            selected: key == nilai,
            onSelected: (_) => onUbah(key),
          ),
      ],
    );
  }
}

/// Tombol untuk mendengar pad dari halaman ini. Ditahan = gate/loop berbunyi.
class _TombolDengar extends StatelessWidget {
  const _TombolDengar({required this.audio, required this.pad});

  final MesinAudio audio;
  final int pad;

  @override
  Widget build(BuildContext context) {
    final t = MxmTokens.of(context);
    return Listener(
      onPointerDown: (_) => audio.ketuk(pad, 1),
      onPointerUp: (_) => audio.lepas(pad),
      onPointerCancel: (_) => audio.lepas(pad),
      child: Container(
        key: const ValueKey('dengar'),
        height: 48,
        alignment: Alignment.center,
        decoration: BoxDecoration(
          color: t.keyDark,
          borderRadius: BorderRadius.circular(6),
        ),
        child: Text(
          'TAHAN UNTUK MENDENGAR',
          style: TextStyle(
            color: t.legendOnDark,
            fontSize: 12,
            fontWeight: FontWeight.w600,
            letterSpacing: 1,
          ),
        ),
      ),
    );
  }
}
