import 'package:flutter/material.dart';

import '../design_system/tokens.dart';

/// Tema Material yang dibangun dari token MXM (ADR-0011).
///
/// Material dipakai hanya sebagai kerangka (teks, navigasi). Komponen MXM
/// sendiri (pad, kenop, layar segmen) membaca [MxmTokens] secara langsung.
ThemeData mxmTheme(MxmTokens t, Brightness brightness) {
  final skema =
      ColorScheme.fromSeed(
        seedColor: t.keyAccent,
        brightness: brightness,
      ).copyWith(
        primary: t.keyAccent,
        onPrimary: t.legendOnAccent,
        surface: t.body,
        onSurface: t.legendOnLight,
        onSurfaceVariant: t.textMuted,
        outline: t.bodyEdge,
      );
  return ThemeData(
    brightness: brightness,
    colorScheme: skema,
    scaffoldBackgroundColor: t.body,
    extensions: [t],
  );
}
