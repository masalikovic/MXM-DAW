//! Fungsi yang bisa dipanggil dari Dart.
//!
//! Setiap fungsi publik di modul ini dibaca oleh `flutter_rust_bridge_codegen`
//! dan diberi pasangan Dart di `app/lib/engine/bridge/` (ADR-0004). Modul ini
//! hanya lapisan tipis: logika tetap di `daw-core` (ADR-0002).

/// Versi mesin, mis. `"0.1.0"`.
///
/// Ditandai `sync`: Dart memanggilnya langsung di thread UI tanpa menunggu
/// `Future`. Aman karena fungsinya hanya membaca konstanta yang ditanam
/// compiler (lihat `daw_core::engine_version`).
#[flutter_rust_bridge::frb(sync)]
#[must_use]
pub fn engine_version() -> String {
    daw_core::engine_version().to_owned()
}

#[cfg(test)]
mod tests {
    #[test]
    fn versi_lewat_ffi_sama_dengan_versi_inti() {
        assert_eq!(super::engine_version(), daw_core::engine_version());
    }
}
