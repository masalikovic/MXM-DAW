//! Jembatan mesin MXM-DAW ke Flutter (ADR-0004).
//!
//! `api` berisi fungsi yang kita tulis sendiri. `frb_generated` dibuat ulang
//! oleh `flutter_rust_bridge_codegen generate` (dijalankan dari folder `app/`)
//! dan tidak boleh diedit tangan; CI memeriksa bahwa hasilnya tidak basi.

pub mod api;
mod audio;

// Kode hasil generator tidak mengikuti lint proyek kita.
#[allow(
    clippy::all,
    clippy::pedantic,
    missing_docs,
    unsafe_op_in_unsafe_fn,
    non_camel_case_types
)]
mod frb_generated;
