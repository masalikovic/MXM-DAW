// Adapter C++ mesin MXM-DAW (ADR-0003).
//
// Di M0 berkas ini sengaja hampir kosong: tujuannya membuktikan bentuk build
// "Rust staticlib + C++ → satu libmxm_engine.so". Di M1, adapter ini membuka
// stream audio Oboe dan meneruskan setiap callback ke fungsi render Rust.
// Tidak ada DSP di C++; semua logika mesin ada di Rust.

extern "C" {

// Versi antarmuka adapter, naik bila cara C++ memanggil Rust berubah.
int mxm_engine_adapter_abi_version() { return 1; }

}  // extern "C"
