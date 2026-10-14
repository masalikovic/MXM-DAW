import 'dart:ui' show Color;

import 'package:flutter_test/flutter_test.dart';
import 'package:mxmdaw/design_system/contrast.dart';

void main() {
  const hitam = Color(0xFF000000);
  const putih = Color(0xFFFFFFFF);

  test('hitam dan putih memberi kontras maksimum 21:1', () {
    expect(contrastRatio(hitam, putih), closeTo(21, 1e-9));
  });

  test('warna yang sama memberi kontras minimum 1:1', () {
    const oranye = Color(0xFFFF8A3D);
    expect(contrastRatio(oranye, oranye), closeTo(1, 1e-9));
  });

  test('urutan warna tidak berpengaruh', () {
    const tinta = Color(0xFF0E0E0E);
    const cream = Color(0xFFF5EBD7);
    expect(contrastRatio(tinta, cream), contrastRatio(cream, tinta));
  });

  test('tinta di atas oranye = 8,2:1, sama dengan angka di rencana §5.2', () {
    const tinta = Color(0xFF0E0E0E);
    const oranye = Color(0xFFFF8A3D);
    expect(contrastRatio(tinta, oranye), closeTo(8.23, 0.01));
  });

  test('luminans hitam 0 dan putih 1', () {
    expect(relativeLuminance(hitam), 0);
    expect(relativeLuminance(putih), closeTo(1, 1e-9));
  });
}
