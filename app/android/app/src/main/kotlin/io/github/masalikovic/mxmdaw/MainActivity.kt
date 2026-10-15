package io.github.masalikovic.mxmdaw

import android.app.Activity
import android.content.Intent
import android.net.Uri
import android.os.Handler
import android.os.Looper
import android.provider.OpenableColumns
import io.flutter.embedding.android.FlutterActivity
import io.flutter.embedding.engine.FlutterEngine
import io.flutter.plugin.common.MethodChannel
import java.util.concurrent.Executors

/**
 * Activity utama. Selain menjalankan Flutter, ia menyediakan kanal
 * `mxmdaw/berkas` untuk memilih berkas audio lewat Storage Access Framework
 * (SAF). Pengguna memilih satu berkas, Android memberi izin baca untuk berkas
 * itu saja, jadi aplikasi tidak butuh izin penyimpanan (rencana §4.2,
 * ADR-0006). Isi berkas dikirim ke Dart sebagai byte, lalu dibaca Rust.
 */
class MainActivity : FlutterActivity() {
    private var hasilTertunda: MethodChannel.Result? = null
    private val pekerja = Executors.newSingleThreadExecutor()
    private val utama = Handler(Looper.getMainLooper())

    override fun configureFlutterEngine(flutterEngine: FlutterEngine) {
        super.configureFlutterEngine(flutterEngine)
        MethodChannel(flutterEngine.dartExecutor.binaryMessenger, KANAL)
            .setMethodCallHandler { call, result ->
                when (call.method) {
                    "pilihAudio" -> pilihAudio(result)
                    // Folder data pribadi aplikasi: tidak butuh izin, ikut
                    // terhapus saat aplikasi di-uninstall (ADR-0016).
                    "folderProyek" -> result.success(java.io.File(filesDir, "proyek").absolutePath)
                    else -> result.notImplemented()
                }
            }
    }

    private fun pilihAudio(result: MethodChannel.Result) {
        if (hasilTertunda != null) {
            result.error("SIBUK", "pemilih berkas sudah terbuka", null)
            return
        }
        hasilTertunda = result
        val intent = Intent(Intent.ACTION_OPEN_DOCUMENT).apply {
            addCategory(Intent.CATEGORY_OPENABLE)
            // Sebagian pengelola berkas memberi WAV jenis yang berbeda-beda,
            // jadi semua audio ditampilkan; Rust yang menolak bila bukan WAV.
            type = "audio/*"
        }
        @Suppress("DEPRECATION")
        startActivityForResult(intent, KODE_PILIH)
    }

    @Deprecated("Activity Result API butuh ComponentActivity; FlutterActivity bukan")
    override fun onActivityResult(requestCode: Int, resultCode: Int, data: Intent?) {
        @Suppress("DEPRECATION")
        super.onActivityResult(requestCode, resultCode, data)
        if (requestCode != KODE_PILIH) return
        val result = hasilTertunda ?: return
        hasilTertunda = null
        val uri = data?.data
        if (resultCode != Activity.RESULT_OK || uri == null) {
            result.success(null) // dibatalkan pengguna
            return
        }
        // Membaca berkas besar di thread utama bisa membuat aplikasi macet (ANR).
        pekerja.execute {
            try {
                val (nama, ukuran) = infoBerkas(uri)
                if (ukuran > BATAS_BYTE) {
                    utama.post {
                        result.error("BESAR", "berkas lebih dari ${BATAS_BYTE / 1_000_000} MB", null)
                    }
                    return@execute
                }
                val isi = contentResolver.openInputStream(uri)?.use { it.readBytes() }
                    ?: throw IllegalStateException("berkas tidak bisa dibuka")
                utama.post { result.success(mapOf("nama" to nama, "isi" to isi)) }
            } catch (e: Exception) {
                utama.post { result.error("BACA", e.message ?: e.toString(), null) }
            }
        }
    }

    private fun infoBerkas(uri: Uri): Pair<String, Long> {
        contentResolver.query(uri, null, null, null, null)?.use { c ->
            if (c.moveToFirst()) {
                val iNama = c.getColumnIndex(OpenableColumns.DISPLAY_NAME)
                val iUkuran = c.getColumnIndex(OpenableColumns.SIZE)
                val nama = if (iNama >= 0) c.getString(iNama) else null
                val ukuran = if (iUkuran >= 0 && !c.isNull(iUkuran)) c.getLong(iUkuran) else -1L
                return Pair(nama ?: "sampel.wav", ukuran)
            }
        }
        return Pair("sampel.wav", -1L)
    }

    override fun onDestroy() {
        pekerja.shutdown()
        super.onDestroy()
    }

    private companion object {
        const val KANAL = "mxmdaw/berkas"
        const val KODE_PILIH = 4207
        /** WAV 2 menit stereo 48 kHz 24-bit ≈ 35 MB; Rust tetap menolak > 2 menit. */
        const val BATAS_BYTE = 64_000_000L
    }
}
