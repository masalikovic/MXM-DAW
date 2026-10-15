import 'bridge/api.dart' as rust;

/// Papan klip salin/tempel (rencana §4.3), hanya di memori; tidak ikut
/// disimpan ke proyek.
///
/// Suara dan grup disalin Rust saat **ditempel** (sampel dipakai bersama,
/// tidak digandakan), jadi yang dicatat di sini hanya nomor sumbernya. Pola
/// disalin sebagai isi, supaya bisa ditempel ke grup mana pun.
class PapanKlip {
  /// Pad global sumber suara.
  int? pad;

  /// Grup sumber (16 suara + pola).
  int? grup;

  /// Not dan panjang pola yang disalin.
  ({List<rust.NoteData> not, int bars})? pola;
}
