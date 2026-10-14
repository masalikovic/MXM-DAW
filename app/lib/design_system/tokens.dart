import 'package:flutter/material.dart';

/// Token warna MXM-DAW (rencana §5.2, ADR-0011).
///
/// Widget tidak boleh menulis kode warna sendiri. Semua warna diambil dari
/// sini lewat `MxmTokens.of(context)`, supaya tema terang dan gelap selalu
/// konsisten dan setiap pasangan teks/latar bisa diperiksa oleh tes kontras.
@immutable
class MxmTokens extends ThemeExtension<MxmTokens> {
  const MxmTokens({
    required this.body,
    required this.bodyEdge,
    required this.keyDark,
    required this.keyLight,
    required this.keyAccent,
    required this.legendOnDark,
    required this.legendOnLight,
    required this.legendOnAccent,
    required this.displayBg,
    required this.displayOn,
    required this.displayAccent,
    required this.displayOff,
    required this.textMuted,
    required this.groupA,
    required this.groupB,
    required this.groupC,
    required this.groupD,
  });

  // Palet dasar MXM.
  static const _tinta = Color(0xFF0E0E0E);
  static const _cream = Color(0xFFF5EBD7);
  static const _oranye = Color(0xFFFF8A3D);

  // Warna grup sama di kedua tema dan selalu tampil bersama huruf A–D.
  // C dan D ditetapkan pemilik di ADR-0011 (kontras ≥ 3:1 di atas key.dark).
  static const _grupC = Color(0xFF8A857C);
  static const _grupD = Color(0xFFB0612F);

  static const light = MxmTokens(
    body: Color(0xFFECE6DA),
    bodyEdge: Color(0xFFD9D2C4),
    keyDark: Color(0xFF1A1918),
    keyLight: _cream,
    keyAccent: _oranye,
    legendOnDark: _cream,
    legendOnLight: _tinta,
    // Cream di atas oranye terlalu pucat, jadi tulisan di tombol oranye tinta.
    legendOnAccent: _tinta,
    displayBg: Color(0xFF0B0B0B),
    displayOn: _cream,
    displayAccent: _oranye,
    // Cream 7 % (alfa 0x12 = 18/255): segmen padam yang samar terlihat.
    displayOff: Color(0x12F5EBD7),
    textMuted: Color(0xFF645F57),
    groupA: _oranye,
    groupB: _cream,
    groupC: _grupC,
    groupD: _grupD,
  );

  static const dark = MxmTokens(
    body: Color(0xFF161514),
    bodyEdge: Color(0xFF24221F),
    keyDark: Color(0xFF2A2826),
    keyLight: Color(0xFF3A3733),
    keyAccent: _oranye,
    legendOnDark: _cream,
    legendOnLight: _cream,
    legendOnAccent: _tinta,
    displayBg: Color(0xFF050505),
    displayOn: _cream,
    displayAccent: _oranye,
    // Cream 6 % (alfa 0x0F = 15/255).
    displayOff: Color(0x0FF5EBD7),
    textMuted: Color(0xFF8A857C),
    groupA: _oranye,
    groupB: _cream,
    groupC: _grupC,
    groupD: _grupD,
  );

  /// Panel utama.
  final Color body;

  /// Garis panel, celah antar-modul.
  final Color bodyEdge;

  /// Pad angka, tombol fungsi.
  final Color keyDark;

  /// Tombol grup, tombol sekunder.
  final Color keyLight;

  /// REC, kenop utama.
  final Color keyAccent;

  /// Tulisan di tombol gelap.
  final Color legendOnDark;

  /// Tulisan di tombol terang (juga teks utama di atas [body]).
  final Color legendOnLight;

  /// Tulisan di tombol oranye.
  final Color legendOnAccent;

  /// Latar layar segmen.
  final Color displayBg;

  /// Segmen menyala.
  final Color displayOn;

  /// Ikon aktif, angka penting di layar segmen.
  final Color displayAccent;

  /// Segmen padam (transparan, dicampur ke atas [displayBg]).
  final Color displayOff;

  /// Label kecil di panel.
  final Color textMuted;

  /// Warna grup A (LED dan garis atas pad), selalu bersama hurufnya.
  final Color groupA;

  /// Warna grup B.
  final Color groupB;

  /// Warna grup C.
  final Color groupC;

  /// Warna grup D.
  final Color groupD;

  /// Warna keempat grup, urut A–D.
  List<Color> get groups => [groupA, groupB, groupC, groupD];

  /// Token tema yang sedang aktif.
  static MxmTokens of(BuildContext context) {
    return Theme.of(context).extension<MxmTokens>()!;
  }

  @override
  MxmTokens copyWith({
    Color? body,
    Color? bodyEdge,
    Color? keyDark,
    Color? keyLight,
    Color? keyAccent,
    Color? legendOnDark,
    Color? legendOnLight,
    Color? legendOnAccent,
    Color? displayBg,
    Color? displayOn,
    Color? displayAccent,
    Color? displayOff,
    Color? textMuted,
    Color? groupA,
    Color? groupB,
    Color? groupC,
    Color? groupD,
  }) {
    return MxmTokens(
      body: body ?? this.body,
      bodyEdge: bodyEdge ?? this.bodyEdge,
      keyDark: keyDark ?? this.keyDark,
      keyLight: keyLight ?? this.keyLight,
      keyAccent: keyAccent ?? this.keyAccent,
      legendOnDark: legendOnDark ?? this.legendOnDark,
      legendOnLight: legendOnLight ?? this.legendOnLight,
      legendOnAccent: legendOnAccent ?? this.legendOnAccent,
      displayBg: displayBg ?? this.displayBg,
      displayOn: displayOn ?? this.displayOn,
      displayAccent: displayAccent ?? this.displayAccent,
      displayOff: displayOff ?? this.displayOff,
      textMuted: textMuted ?? this.textMuted,
      groupA: groupA ?? this.groupA,
      groupB: groupB ?? this.groupB,
      groupC: groupC ?? this.groupC,
      groupD: groupD ?? this.groupD,
    );
  }

  /// Peralihan halus saat tema berganti terang ↔ gelap.
  @override
  MxmTokens lerp(MxmTokens? other, double t) {
    if (other == null) return this;
    Color c(Color a, Color b) => Color.lerp(a, b, t)!;
    return MxmTokens(
      body: c(body, other.body),
      bodyEdge: c(bodyEdge, other.bodyEdge),
      keyDark: c(keyDark, other.keyDark),
      keyLight: c(keyLight, other.keyLight),
      keyAccent: c(keyAccent, other.keyAccent),
      legendOnDark: c(legendOnDark, other.legendOnDark),
      legendOnLight: c(legendOnLight, other.legendOnLight),
      legendOnAccent: c(legendOnAccent, other.legendOnAccent),
      displayBg: c(displayBg, other.displayBg),
      displayOn: c(displayOn, other.displayOn),
      displayAccent: c(displayAccent, other.displayAccent),
      displayOff: c(displayOff, other.displayOff),
      textMuted: c(textMuted, other.textMuted),
      groupA: c(groupA, other.groupA),
      groupB: c(groupB, other.groupB),
      groupC: c(groupC, other.groupC),
      groupD: c(groupD, other.groupD),
    );
  }
}
