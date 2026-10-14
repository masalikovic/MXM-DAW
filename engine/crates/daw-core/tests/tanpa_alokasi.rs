//! Aturan thread audio (rencana §8.4, §12): `Engine::render` tidak boleh
//! mengalokasi memori. `AllocDisabler` membuat setiap alokasi di dalam
//! `assert_no_alloc` menghentikan proses (abort), jadi tes ini gagal bila aturan
//! dilanggar. Karena abort tidak bisa ditangkap, bukti bahwa tes ini bisa gagal
//! dicatat di PR M1 (mutasi: `Vec` dibuat di dalam `render`).

use assert_no_alloc::{AllocDisabler, assert_no_alloc};
use daw_core::engine::Engine;

#[global_allocator]
static PENGALOKASI: AllocDisabler = AllocDisabler;

#[test]
fn render_tanpa_alokasi() {
    let (mut mesin, mut kirim) = Engine::new(48_000);
    let mut blok = vec![0.0_f32; 256 * 2];
    for i in 0..500 {
        if i % 7 == 0 {
            kirim.hit(1.0);
        }
        assert_no_alloc(|| mesin.render(&mut blok, 2));
    }
}
