// Adapter C++ mesin MXM-DAW (ADR-0003).
//
// Tugasnya hanya tiga: membuka stream audio rendah-latensi lewat Oboe,
// meneruskan setiap callback ke fungsi render Rust (`mxm_engine_render`), dan
// melaporkan angka kesehatan (latensi, underrun, beban callback). Tidak ada
// DSP di sini; semua bunyi dibuat di Rust (daw-core).
//
// Aturan thread audio (rencana §8.4): `onAudioReady` tidak mengalokasi, tidak
// mengunci, dan tidak menulis log. Angka kesehatan dikirim lewat atomik.

#include <oboe/Oboe.h>

#include <atomic>
#include <chrono>
#include <cstdint>
#include <memory>
#include <mutex>

extern "C" void mxm_engine_render(float* out, int32_t frames, int32_t channels);

namespace {

constexpr int32_t kSampleRate = 48000;  // ADR-0013: mesin selalu 48 kHz
constexpr int32_t kChannels = 2;

class Adapter : public oboe::AudioStreamDataCallback,
                public oboe::AudioStreamErrorCallback {
 public:
  int32_t start() {
    std::lock_guard<std::mutex> kunci(lock_);
    if (stream_) return 0;
    return static_cast<int32_t>(openAndStartLocked());
  }

  void stop() {
    std::lock_guard<std::mutex> kunci(lock_);
    closeLocked();
  }

  oboe::DataCallbackResult onAudioReady(oboe::AudioStream* stream, void* data,
                                        int32_t frames) override {
    const auto mulai = std::chrono::steady_clock::now();
    if (tuner_) tuner_->tune();
    mxm_engine_render(static_cast<float*>(data), frames,
                      stream->getChannelCount());
    const double detik =
        std::chrono::duration<double>(std::chrono::steady_clock::now() - mulai)
            .count();
    const double periode =
        static_cast<double>(frames) / static_cast<double>(stream->getSampleRate());
    const float beban = static_cast<float>(detik / periode);
    // Rata-rata bergerak eksponensial dan nilai tertinggi beban callback.
    const float rata = loadAvg_.load(std::memory_order_relaxed);
    loadAvg_.store(rata + 0.02f * (beban - rata), std::memory_order_relaxed);
    if (beban > loadMax_.load(std::memory_order_relaxed)) {
      loadMax_.store(beban, std::memory_order_relaxed);
    }
    return oboe::DataCallbackResult::Continue;
  }

  // Perangkat keluaran berganti (mis. headset dicabut): buka ulang stream.
  void onErrorAfterClose(oboe::AudioStream* /*stream*/,
                         oboe::Result error) override {
    if (error != oboe::Result::ErrorDisconnected) return;
    std::lock_guard<std::mutex> kunci(lock_);
    if (!stream_) return;  // sudah dihentikan pengguna
    stream_.reset();
    tuner_.reset();
    if (openAndStartLocked() == oboe::Result::OK) {
      restarts_.fetch_add(1, std::memory_order_relaxed);
    }
  }

  struct Kesehatan {
    int32_t running;
    int32_t sampleRate;
    int32_t channels;
    int32_t framesPerBurst;
    int32_t bufferSize;
    int32_t bufferCapacity;
    int32_t xruns;
    int32_t exclusive;
    int32_t aaudio;
    int32_t restarts;
    double latencyMs;
    double loadAvg;
    double loadMax;
  };

  void health(Kesehatan* out) {
    std::lock_guard<std::mutex> kunci(lock_);
    *out = Kesehatan{};
    out->latencyMs = -1.0;
    out->xruns = -1;
    out->restarts = restarts_.load(std::memory_order_relaxed);
    out->loadAvg = loadAvg_.load(std::memory_order_relaxed);
    out->loadMax = loadMax_.load(std::memory_order_relaxed);
    if (!stream_) return;
    out->running = stream_->getState() == oboe::StreamState::Started;
    out->sampleRate = stream_->getSampleRate();
    out->channels = stream_->getChannelCount();
    out->framesPerBurst = stream_->getFramesPerBurst();
    out->bufferSize = stream_->getBufferSizeInFrames();
    out->bufferCapacity = stream_->getBufferCapacityInFrames();
    out->exclusive = stream_->getSharingMode() == oboe::SharingMode::Exclusive;
    out->aaudio = stream_->getAudioApi() == oboe::AudioApi::AAudio;
    auto xrun = stream_->getXRunCount();
    if (xrun) out->xruns = xrun.value();
    auto latensi = stream_->calculateLatencyMillis();
    if (latensi) out->latencyMs = latensi.value();
  }

 private:
  oboe::Result openAndStartLocked() {
    oboe::AudioStreamBuilder builder;
    builder.setDirection(oboe::Direction::Output)
        ->setPerformanceMode(oboe::PerformanceMode::LowLatency)
        ->setSharingMode(oboe::SharingMode::Exclusive)  // jatuh ke Shared bila ditolak
        ->setFormat(oboe::AudioFormat::Float)
        ->setChannelCount(kChannels)
        ->setSampleRate(kSampleRate)
        ->setSampleRateConversionQuality(oboe::SampleRateConversionQuality::Medium)
        ->setDataCallback(this)
        ->setErrorCallback(this);
    std::shared_ptr<oboe::AudioStream> stream;
    oboe::Result hasil = builder.openStream(stream);
    if (hasil != oboe::Result::OK) return hasil;
    // Buffer mulai dari 2 × burst; LatencyTuner menaikkannya bila underrun
    // (rencana §9).
    stream->setBufferSizeInFrames(stream->getFramesPerBurst() * 2);
    tuner_ = std::make_unique<oboe::LatencyTuner>(*stream);
    loadAvg_.store(0.0f, std::memory_order_relaxed);
    loadMax_.store(0.0f, std::memory_order_relaxed);
    hasil = stream->requestStart();
    if (hasil != oboe::Result::OK) {
      stream->close();
      tuner_.reset();
      return hasil;
    }
    stream_ = stream;
    return oboe::Result::OK;
  }

  void closeLocked() {
    if (!stream_) return;
    // stop() dan close() menunggu callback terakhir selesai; sesudahnya Rust
    // aman membebaskan mesin (ADR-0013).
    stream_->stop();
    stream_->close();
    stream_.reset();
    tuner_.reset();
  }

  std::mutex lock_;
  std::shared_ptr<oboe::AudioStream> stream_;
  std::unique_ptr<oboe::LatencyTuner> tuner_;
  std::atomic<float> loadAvg_{0.0f};
  std::atomic<float> loadMax_{0.0f};
  std::atomic<int32_t> restarts_{0};
};

Adapter gAdapter;

}  // namespace

extern "C" {

// Versi antarmuka adapter, naik bila cara C++ dan Rust saling memanggil berubah.
int mxm_engine_adapter_abi_version() { return 2; }

// 0 = berhasil; negatif = kode oboe::Result.
int32_t mxm_audio_start() { return gAdapter.start(); }

void mxm_audio_stop() { gAdapter.stop(); }

// Tata letak harus sama dengan `KesehatanC` di daw-ffi/src/audio.rs.
void mxm_audio_health(Adapter::Kesehatan* out) { gAdapter.health(out); }

}  // extern "C"
