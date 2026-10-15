import 'package:flutter/material.dart';

import '../design_system/tokens.dart';

/// Huruf grup 0..3.
const hurufGrup = ['A', 'B', 'C', 'D'];

/// Kolom tombol grup A–D di kiri pad (rencana §6.1, §7): ketuk = pilih grup,
/// tahan = mute/buka polanya.
class KolomGrup extends StatelessWidget {
  const KolomGrup({
    required this.grup,
    required this.mute,
    required this.onPilih,
    required this.onTahan,
    this.aktif = true,
    this.tinggi = 56,
    super.key,
  });

  /// Tinggi tombol: sama dengan ukuran pad, supaya kolom setinggi kisi pad.
  final double tinggi;

  /// Grup yang dipilih (0..3).
  final int grup;

  /// Grup yang di-mute.
  final List<bool> mute;
  final ValueChanged<int> onPilih;
  final ValueChanged<int> onTahan;
  final bool aktif;

  static const lebar = 44.0;

  @override
  Widget build(BuildContext context) {
    final t = MxmTokens.of(context);
    final warna = [t.groupA, t.groupB, t.groupC, t.groupD];
    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        for (var g = 0; g < 4; g++)
          Padding(
            padding: EdgeInsets.only(top: g == 0 ? 0 : 8),
            child: Semantics(
              button: true,
              selected: g == grup,
              label:
                  'Grup ${hurufGrup[g]}'
                  '${g < mute.length && mute[g] ? ', di-mute' : ''}',
              child: Material(
                color: g == grup ? t.keyDark : t.bodyEdge,
                borderRadius: BorderRadius.circular(6),
                child: InkWell(
                  key: ValueKey('grup-$g'),
                  borderRadius: BorderRadius.circular(6),
                  onTap: aktif ? () => onPilih(g) : null,
                  onLongPress: aktif ? () => onTahan(g) : null,
                  child: SizedBox(
                    width: lebar,
                    height: tinggi,
                    child: Column(
                      mainAxisAlignment: MainAxisAlignment.center,
                      children: [
                        Text(
                          hurufGrup[g],
                          style: TextStyle(
                            color: g == grup ? warna[g] : t.legendOnLight,
                            fontSize: 18,
                            fontWeight: FontWeight.w700,
                          ),
                        ),
                        if (g < mute.length && mute[g])
                          Text(
                            'MUTE',
                            key: ValueKey('mute-$g'),
                            style: TextStyle(
                              color: g == grup ? t.legendOnDark : t.textMuted,
                              fontSize: 8,
                              fontWeight: FontWeight.w600,
                            ),
                          ),
                      ],
                    ),
                  ),
                ),
              ),
            ),
          ),
      ],
    );
  }
}
