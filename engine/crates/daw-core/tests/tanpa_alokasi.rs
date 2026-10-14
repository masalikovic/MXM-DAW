//! Aturan thread audio (rencana §8.4, §12): `Engine::render` tidak boleh
//! mengalokasi memori. `AllocDisabler` membuat setiap alokasi (dan
//! pembebasan) di dalam `assert_no_alloc` menghentikan proses (abort), jadi tes
//! ini gagal bila aturan dilanggar. Karena abort tidak bisa ditangkap, bukti
//! bahwa tes ini bisa gagal dicatat di PR (mutasi).

use assert_no_alloc::{AllocDisabler, assert_no_alloc};
use daw_core::engine::Engine;
use daw_core::pad::{FilterMode, PlayMode};
use daw_core::sampel::Sampel;

#[global_allocator]
static PENGALOKASI: AllocDisabler = AllocDisabler;

#[test]
fn render_tanpa_alokasi() {
    let (mut mesin, mut kirim) = Engine::new(48_000);
    let mut blok = vec![0.0_f32; 256 * 2];
    let mut besar = vec![0.0_f32; 1_000 * 2];
    let pendek = vec![0.1_f32; 4_800];
    for i in 0..600_usize {
        // Semua jenis event, termasuk ganti sampel (sampel lama harus
        // dikembalikan ke sini, bukan dibebaskan di thread audio).
        kirim.hit(i % 16, 1.0);
        if i % 5 == 0 {
            kirim.release(i % 16);
        }
        if i % 50 == 0 {
            let mut p = *kirim.pad(i % 16).unwrap();
            p.play_mode = PlayMode::Loop;
            p.filter.mode = FilterMode::LowPass;
            p.drive = 0.5;
            kirim.set_pad(i % 16, &p);
        }
        if i % 97 == 0 {
            kirim.set_sample(i % 16, "SINUS", Sampel::from_f32(&pendek, 1, 44_100));
        }
        assert_no_alloc(|| mesin.render(&mut blok, 2));
        // Blok lebih besar dari buffer campur internal: dipecah, tetap tanpa alokasi.
        assert_no_alloc(|| mesin.render(&mut besar, 2));
    }
}
