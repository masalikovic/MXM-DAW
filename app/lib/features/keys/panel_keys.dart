import 'package:flutter/material.dart';

import '../../design_system/tokens.dart';
import 'keys.dart';

/// Baris KEYS dan note repeat di atas transport: KEYS hidup/mati, oktaf,
/// skala, dan kecepatan ulang.
class PanelKeys extends StatelessWidget {
  const PanelKeys({
    required this.aktif,
    required this.keys,
    required this.oktaf,
    required this.skala,
    required this.rate,
    required this.onKeys,
    required this.onOktaf,
    required this.onSkala,
    required this.onRate,
    super.key,
  });

  final bool aktif;
  final bool keys;
  final int oktaf;
  final Skala skala;

  /// Jarak note repeat (tick); 0 = mati.
  final int rate;
  final VoidCallback onKeys;
  final ValueChanged<int> onOktaf;
  final VoidCallback onSkala;
  final VoidCallback onRate;

  @override
  Widget build(BuildContext context) {
    final t = MxmTokens.of(context);
    final labelRate = pilihanUlang.entries
        .firstWhere(
          (e) => e.value == rate,
          orElse: () => pilihanUlang.entries.first,
        )
        .key;
    Widget tombol(
      String kunci,
      String label, {
      VoidCallback? onTekan,
      bool nyala = false,
      int lebar = 1,
    }) => Expanded(
      flex: lebar,
      child: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 3),
        child: Material(
          color: onTekan != null ? t.keyDark : t.bodyEdge,
          borderRadius: BorderRadius.circular(6),
          child: InkWell(
            key: ValueKey(kunci),
            borderRadius: BorderRadius.circular(6),
            onTap: onTekan,
            child: SizedBox(
              height: 40,
              child: Center(
                child: Text(
                  label,
                  style: TextStyle(
                    color: nyala ? t.groupA : t.legendOnDark,
                    fontSize: 11,
                    fontWeight: FontWeight.w600,
                  ),
                ),
              ),
            ),
          ),
        ),
      ),
    );
    return Row(
      children: [
        tombol('keys', 'KEYS', nyala: keys, onTekan: aktif ? onKeys : null),
        tombol(
          'oktaf-kurang',
          'OKT −',
          onTekan: aktif && keys && oktaf > Keys.oktafMin
              ? () => onOktaf(oktaf - 1)
              : null,
        ),
        tombol(
          'oktaf-tambah',
          'OKT +',
          onTekan: aktif && keys && oktaf < Keys.oktafMaks
              ? () => onOktaf(oktaf + 1)
              : null,
        ),
        tombol('skala', skala.label, onTekan: aktif && keys ? onSkala : null),
        tombol(
          'ulang',
          'RPT $labelRate',
          lebar: 2,
          nyala: rate > 0,
          onTekan: aktif ? onRate : null,
        ),
      ],
    );
  }
}
