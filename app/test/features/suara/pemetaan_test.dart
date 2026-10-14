import 'package:flutter_test/flutter_test.dart';
import 'package:mxmdaw/features/suara/pemetaan.dart';

void main() {
  test('waktu: 1 ms … 10 s, bolak-balik', () {
    expect(Pemetaan.waktu(0), closeTo(0.001, 1e-9));
    expect(Pemetaan.waktu(1), closeTo(10, 1e-9));
    for (final detik in [0.001, 0.02, 0.35, 2.0]) {
      expect(
        Pemetaan.waktu(Pemetaan.waktuKeKenop(detik)),
        closeTo(detik, 1e-6),
      );
    }
  });

  test('pitch dipecah menjadi semitone dan cent', () {
    expect(Pemetaan.pecahPitch(7), (7, 0));
    expect(Pemetaan.pecahPitch(-3.25), (-3, -25));
    expect(Pemetaan.semitone(0.5), 0);
    expect(Pemetaan.semitone(1), 24);
    expect(Pemetaan.semitone(Pemetaan.semitoneKeKenop(-12)), -12);
    expect(Pemetaan.cent(Pemetaan.centKeKenop(33)), 33);
  });

  test('pan dan cutoff sama dengan mesin Rust', () {
    expect(Pemetaan.pan(0.5), 0);
    expect(Pemetaan.panKeKenop(-1), 0);
    expect(Pemetaan.cutoffHz(0), closeTo(20, 1e-9));
    expect(Pemetaan.cutoffHz(1), closeTo(20000, 1e-6));
  });

  test('teks yang dibaca manusia', () {
    expect(Pemetaan.teksWaktu(0.02), '20 ms');
    expect(Pemetaan.teksWaktu(1.5), '1.50 s');
    expect(Pemetaan.teksPan(0), 'C');
    expect(Pemetaan.teksPan(-0.4), 'L40');
    expect(Pemetaan.teksPan(1), 'R100');
    expect(Pemetaan.teksBertanda(7, 'st'), '+7 st');
    expect(Pemetaan.teksBertanda(-3, 'st'), '-3 st');
    expect(Pemetaan.teksHz(632), '632 Hz');
    expect(Pemetaan.teksHz(1200), '1.2 kHz');
  });
}
