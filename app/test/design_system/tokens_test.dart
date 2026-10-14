import 'package:flutter/material.dart';
import 'package:flutter_test/flutter_test.dart';
import 'package:mxmdaw/design_system/contrast.dart';
import 'package:mxmdaw/design_system/tokens.dart';

/// Batas WCAG 2.x tingkat AA.
const minTeks = 4.5; // teks biasa
const minGrafis = 3.0; // elemen grafis: LED, garis grup

String hex(Color c) =>
    '#${c.toARGB32().toRadixString(16).padLeft(8, '0').toUpperCase()}';

void main() {
  const tema = {'terang': MxmTokens.light, 'gelap': MxmTokens.dark};

  for (final MapEntry(key: nama, value: t) in tema.entries) {
    group('tema $nama', () {
      // Setiap pasangan teks/latar yang dipakai antarmuka (rencana §5.2).
      // Pasangan baru wajib ditambahkan ke sini.
      final pasanganTeks = {
        'legend.onDark di atas key.dark': (t.legendOnDark, t.keyDark),
        'legend.onLight di atas key.light': (t.legendOnLight, t.keyLight),
        'legend.onAccent di atas key.accent': (t.legendOnAccent, t.keyAccent),
        'legend.onLight di atas body': (t.legendOnLight, t.body),
        'text.muted di atas body': (t.textMuted, t.body),
        'display.on di atas display.bg': (t.displayOn, t.displayBg),
        'display.accent di atas display.bg': (t.displayAccent, t.displayBg),
      };

      for (final MapEntry(key: pasangan, value: (teks, latar))
          in pasanganTeks.entries) {
        test('$pasangan ≥ 4,5:1', () {
          final rasio = contrastRatio(teks, latar);
          expect(
            rasio,
            greaterThanOrEqualTo(minTeks),
            reason:
                '${hex(teks)} di atas ${hex(latar)} = '
                '${rasio.toStringAsFixed(2)}:1',
          );
        });
      }

      const huruf = ['A', 'B', 'C', 'D'];
      for (var i = 0; i < 4; i++) {
        test('warna grup ${huruf[i]} di atas key.dark ≥ 3:1', () {
          final rasio = contrastRatio(t.groups[i], t.keyDark);
          expect(
            rasio,
            greaterThanOrEqualTo(minGrafis),
            reason: '${hex(t.groups[i])} = ${rasio.toStringAsFixed(2)}:1',
          );
        });
      }

      test(
        'segmen padam samar: terlihat, tetapi jauh di bawah segmen menyala',
        () {
          final padam = composite(t.displayOff, t.displayBg);
          final rasio = contrastRatio(padam, t.displayBg);
          expect(rasio, greaterThan(1.05));
          expect(rasio, lessThan(1.5));
        },
      );

      test('warna grup A–D berbeda satu sama lain', () {
        expect(t.groups.toSet(), hasLength(4));
      });
    });
  }

  // Penjaga terhadap perubahan tak sengaja: nilai harus sama dengan rencana
  // §5.2, kecuali grup C dan D yang ditetapkan ADR-0011.
  test('nilai token sama dengan rencana §5.2 dan ADR-0011', () {
    const terang = MxmTokens.light;
    const gelap = MxmTokens.dark;
    expect(
      {
        'body': [hex(terang.body), hex(gelap.body)],
        'body.edge': [hex(terang.bodyEdge), hex(gelap.bodyEdge)],
        'key.dark': [hex(terang.keyDark), hex(gelap.keyDark)],
        'key.light': [hex(terang.keyLight), hex(gelap.keyLight)],
        'key.accent': [hex(terang.keyAccent), hex(gelap.keyAccent)],
        'legend.onDark': [hex(terang.legendOnDark), hex(gelap.legendOnDark)],
        'legend.onLight': [hex(terang.legendOnLight), hex(gelap.legendOnLight)],
        'legend.onAccent': [
          hex(terang.legendOnAccent),
          hex(gelap.legendOnAccent),
        ],
        'display.bg': [hex(terang.displayBg), hex(gelap.displayBg)],
        'display.on': [hex(terang.displayOn), hex(gelap.displayOn)],
        'display.accent': [hex(terang.displayAccent), hex(gelap.displayAccent)],
        'display.off': [hex(terang.displayOff), hex(gelap.displayOff)],
        'text.muted': [hex(terang.textMuted), hex(gelap.textMuted)],
        'group': [
          terang.groups.map(hex).toList(),
          gelap.groups.map(hex).toList(),
        ],
      },
      {
        'body': ['#FFECE6DA', '#FF161514'],
        'body.edge': ['#FFD9D2C4', '#FF24221F'],
        'key.dark': ['#FF1A1918', '#FF2A2826'],
        'key.light': ['#FFF5EBD7', '#FF3A3733'],
        'key.accent': ['#FFFF8A3D', '#FFFF8A3D'],
        'legend.onDark': ['#FFF5EBD7', '#FFF5EBD7'],
        'legend.onLight': ['#FF0E0E0E', '#FFF5EBD7'],
        'legend.onAccent': ['#FF0E0E0E', '#FF0E0E0E'],
        'display.bg': ['#FF0B0B0B', '#FF050505'],
        'display.on': ['#FFF5EBD7', '#FFF5EBD7'],
        'display.accent': ['#FFFF8A3D', '#FFFF8A3D'],
        'display.off': ['#12F5EBD7', '#0FF5EBD7'],
        'text.muted': ['#FF645F57', '#FF8A857C'],
        'group': [
          ['#FFFF8A3D', '#FFF5EBD7', '#FF8A857C', '#FFB0612F'],
          ['#FFFF8A3D', '#FFF5EBD7', '#FF8A857C', '#FFB0612F'],
        ],
      },
    );
  });
}
