import 'dart:async';

import 'package:flutter/material.dart';

import '../design_system/tokens.dart';
import '../engine/audio.dart';
import '../engine/engine.dart';
import 'pad_grid.dart';
import 'suara/halaman_suara.dart';
import 'transport/tap_tempo.dart';
import 'transport/transport.dart';

/// Layar utama (Pocket, rencana §6.1): panel kesehatan mesin, layar segmen,
/// 16 pad, dan transport sequencer.
class Beranda extends StatefulWidget {
  const Beranda({required this.engine, this.audio, super.key});

  final EngineStatus engine;

  /// `null` bila mesin Rust tidak termuat; pad lalu dinonaktifkan.
  final MesinAudio? audio;

  @override
  State<Beranda> createState() => _BerandaState();
}

class _BerandaState extends State<Beranda> with WidgetsBindingObserver {
  Timer? _penyegar;
  AudioHealth? _kesehatan;
  String? _galat;
  Timer? _penyegarLed;
  int _berbunyi = 0;
  List<String> _nama = const [];
  TransportState _status = const TransportState(
    playing: false,
    recording: false,
    positionTicks: 0,
    bar: 1,
    beat: 1,
    step: -1,
    noteCount: 0,
  );
  SeqConfig? _seq;
  final _tapTempo = TapTempo();
  final _jam = Stopwatch()..start();

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);
    if (widget.audio != null) {
      _nama = widget.audio!.namaPad();
      _seq = widget.audio!.pengaturanSeq();
      unawaited(_mulai());
      _penyegar = Timer.periodic(
        const Duration(milliseconds: 500),
        (_) => _segarkan(),
      );
      // LED pad dan posisi transport ±30 kali per detik; hanya digambar ulang
      // bila berubah.
      _penyegarLed = Timer.periodic(
        const Duration(milliseconds: 33),
        (_) => _segarkanLed(),
      );
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _penyegar?.cancel();
    _penyegarLed?.cancel();
    unawaited(widget.audio?.berhenti());
    super.dispose();
  }

  /// Stream berhenti saat aplikasi di latar, supaya hemat baterai dan tidak
  /// memegang perangkat audio (ADR-0009 butir 4, rencana §11).
  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    final audio = widget.audio;
    if (audio == null) return;
    switch (state) {
      case AppLifecycleState.resumed:
        unawaited(_mulai());
      case AppLifecycleState.paused:
      case AppLifecycleState.hidden:
      case AppLifecycleState.detached:
        unawaited(audio.berhenti());
      case AppLifecycleState.inactive:
        break;
    }
  }

  Future<void> _mulai() async {
    try {
      await widget.audio!.mulai();
      if (mounted) setState(() => _galat = null);
    } on Object catch (galat) {
      if (mounted) setState(() => _galat = '$galat');
    }
    _segarkan();
  }

  void _segarkan() {
    final audio = widget.audio;
    if (audio == null || !mounted) return;
    setState(() => _kesehatan = audio.kesehatan());
  }

  void _bukaSuara(int pad) {
    final audio = widget.audio!;
    // Pad yang ditahan dilepas dulu, supaya mode gate/loop tidak terus berbunyi.
    audio.lepas(pad);
    unawaited(
      Navigator.of(context)
          .push(
            MaterialPageRoute<void>(
              builder: (_) => HalamanSuara(
                audio: audio,
                pad: pad,
                nama: pad < _nama.length ? _nama[pad] : '',
              ),
            ),
          )
          // Nama pad bisa berubah setelah impor.
          .then((_) {
            if (mounted) setState(() => _nama = audio.namaPad());
          }),
    );
  }

  void _segarkanLed() {
    final audio = widget.audio;
    if (audio == null || !mounted) return;
    final berbunyi = audio.padBerbunyi();
    final status = audio.statusTransport();
    if (berbunyi != _berbunyi || status != _status) {
      setState(() {
        _berbunyi = berbunyi;
        _status = status;
      });
    }
  }

  void _main() {
    final audio = widget.audio!;
    _status.playing ? audio.hentikan() : audio.mainkan(rekam: false);
    _segarkanLed();
  }

  /// REC saat berhenti: mulai dengan count-in. REC saat bermain: overdub
  /// hidup/mati tanpa berhenti.
  void _rekam() {
    final audio = widget.audio!;
    _status.playing
        ? audio.aturRekam(nyala: !_status.recording)
        : audio.mainkan(rekam: true);
    _segarkanLed();
  }

  void _aturSeq(SeqConfig Function(SeqConfig) ubah) {
    final seq = _seq;
    if (seq == null) return;
    setState(() => _seq = widget.audio!.aturPengaturanSeq(ubah(seq)));
  }

  void _tap() {
    final bpm = _tapTempo.ketuk(_jam.elapsed);
    if (bpm != null) _aturSeq((s) => s.salin(bpm: bpm));
  }

  void _hapusPola() {
    widget.audio!.aturNotPola(const []);
    _segarkanLed();
    ScaffoldMessenger.of(context)
        .showSnackBar(const SnackBar(content: Text('Pola dikosongkan')));
  }

  @override
  Widget build(BuildContext context) {
    final t = MxmTokens.of(context);
    final versi = switch (widget.engine) {
      EngineReady(:final version) => 'MESIN $version',
      EngineUnavailable() => 'MESIN TIDAK TERMUAT',
    };
    return Scaffold(
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
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
              Text(versi, style: TextStyle(color: t.textMuted, fontSize: 13)),
              if (widget.engine case EngineUnavailable(:final reason))
                Text(
                  reason,
                  style: TextStyle(color: t.textMuted, fontSize: 11),
                ),
              const SizedBox(height: 8),
              PanelKesehatan(kesehatan: _kesehatan, galat: _galat),
              if (_seq case final seq?) ...[
                const SizedBox(height: 8),
                LayarSegmen(status: _status, seq: seq),
              ],
              const Spacer(),
              Center(
                child: KisiPad(
                  aktif: widget.audio != null,
                  nama: _nama,
                  berbunyi: _berbunyi,
                  onKetuk: (pad, v) => widget.audio?.ketuk(pad, v),
                  onLepas: (pad) => widget.audio?.lepas(pad),
                  onTahan: widget.audio == null ? null : _bukaSuara,
                ),
              ),
              const Spacer(),
              if (_seq case final seq?)
                PanelTransport(
                  aktif: widget.audio != null,
                  status: _status,
                  seq: seq,
                  onMain: _main,
                  onRekam: _rekam,
                  onSeq: _aturSeq,
                  onTap: _tap,
                  onHapus: _hapusPola,
                ),
            ],
          ),
        ),
      ),
    );
  }
}

/// Panel "kesehatan mesin" (rencana §6.3): angka apa adanya, termasuk yang
/// buruk (prinsip "jujur", §2).
class PanelKesehatan extends StatelessWidget {
  const PanelKesehatan({required this.kesehatan, this.galat, super.key});

  final AudioHealth? kesehatan;
  final String? galat;

  @override
  Widget build(BuildContext context) {
    final t = MxmTokens.of(context);
    final k = kesehatan;
    final baris = <String>[
      if (galat != null) 'AUDIO GAGAL: $galat',
      if (k == null)
        'AUDIO TIDAK TERSEDIA'
      else ...[
        if (!k.running) 'AUDIO BERHENTI',
        'LAT ${k.latencyMs < 0 ? '?' : '${k.latencyMs.toStringAsFixed(1)} ms'}'
            '   BUF ${k.bufferSize}/${k.framesPerBurst}',
        'BEBAN ${_persen(k.loadAvg)} (MAKS ${_persen(k.loadMax)})'
            '   XRUN ${k.xruns < 0 ? '?' : k.xruns}',
        '${k.sampleRate} Hz · ${k.aaudio ? 'AAudio' : 'OpenSL'}'
            ' · ${k.exclusive ? 'EKSKLUSIF' : 'BERSAMA'}'
            '${k.restarts > 0 ? ' · ULANG ${k.restarts}' : ''}',
        'SUARA ${k.voicesActive}   KETUK ${k.hitsPlayed}'
            '${k.hitsDropped > 0 ? '   BUANG ${k.hitsDropped}' : ''}',
      ],
    ];
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 8),
      decoration: BoxDecoration(
        color: t.displayBg,
        borderRadius: BorderRadius.circular(6),
      ),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          for (final b in baris)
            Text(
              b,
              style: TextStyle(
                color: b.startsWith('AUDIO') ? t.displayAccent : t.displayOn,
                fontFamily: 'monospace',
                fontSize: 11,
                height: 1.3,
              ),
            ),
        ],
      ),
    );
  }

  static String _persen(double x) => '${(x * 100).round()}%';
}
