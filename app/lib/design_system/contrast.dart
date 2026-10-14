import 'dart:math' as math;
import 'dart:ui' show Color;

/// Luminans relatif sebuah warna menurut WCAG 2.x (0 = hitam, 1 = putih).
///
/// Langkahnya: setiap kanal sRGB (0..1) diubah ke nilai linear, lalu
/// dijumlahkan dengan bobot kepekaan mata (hijau paling terang, biru paling
/// redup). Kanal alfa diabaikan; warna transparan harus dicampur dulu ke
/// latarnya (lihat [composite]).
double relativeLuminance(Color color) {
  double linear(double c) {
    return c <= 0.04045
        ? c / 12.92
        : math.pow((c + 0.055) / 1.055, 2.4).toDouble();
  }

  return 0.2126 * linear(color.r) +
      0.7152 * linear(color.g) +
      0.0722 * linear(color.b);
}

/// Rasio kontras WCAG antara dua warna, dari 1:1 sampai 21:1.
///
/// Teks biasa butuh ≥ 4,5:1 (AA); elemen grafis seperti LED butuh ≥ 3:1.
double contrastRatio(Color a, Color b) {
  final la = relativeLuminance(a);
  final lb = relativeLuminance(b);
  final terang = math.max(la, lb);
  final gelap = math.min(la, lb);
  return (terang + 0.05) / (gelap + 0.05);
}

/// Warna [foreground] yang transparan, dicampur ke atas [background] yang
/// buram. Hasilnya warna buram yang benar-benar terlihat di layar.
Color composite(Color foreground, Color background) {
  final a = foreground.a;
  double campur(double f, double b) => f * a + b * (1 - a);
  return Color.from(
    alpha: 1,
    red: campur(foreground.r, background.r),
    green: campur(foreground.g, background.g),
    blue: campur(foreground.b, background.b),
  );
}
