plugins {
    id("com.android.application")
    // The Flutter Gradle Plugin must be applied after the Android and Kotlin Gradle plugins.
    id("dev.flutter.flutter-gradle-plugin")
}

android {
    namespace = "io.github.masalikovic.mxmdaw"
    compileSdk = flutter.compileSdkVersion
    // Ditulis eksplisit (sama dengan bawaan Flutter 3.47.7) karena CI memasang
    // NDK yang sama untuk cargo-ndk dengan membaca baris ini.
    ndkVersion = "28.2.13676358"

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    defaultConfig {
        // Application ID tidak boleh diubah lagi (ADR-0009 butir 1, AGENTS.md).
        applicationId = "io.github.masalikovic.mxmdaw"
        // Android 8.1: setiap perangkat yang bisa memasang memakai AAudio (ADR-0008).
        minSdk = 27
        targetSdk = flutter.targetSdkVersion
        // Uses the version code from pubspec.yaml. When using split APKs, 1000 * ABI_VERSION
        // is added automatically by Flutter. (https://developer.android.com/studio/build/configure-apk-splits#configure-APK-versions)
        // You can force using the value of versionCode by specifying the `-P force-version-code-ignoring-abi=true`
        // flag during build.
        versionCode = flutter.versionCode
        versionName = flutter.versionName

        // Hanya arm64-v8a (ADR-0009 butir 8).
        ndk {
            abiFilters += "arm64-v8a"
        }

        // Plugin Gradle Flutter menambahkan ABI lain ke ndk.abiFilters (daftar
        // itu bersifat gabungan), sehingga AGP sempat menjalankan CMake untuk
        // armeabi-v7a. Filter ini membatasi build CMake (libmxm_engine.so) ke
        // arm64-v8a saja; tools/cek-apk.sh memeriksa ABI yang masuk APK.
        externalNativeBuild {
            cmake {
                abiFilters += "arm64-v8a"
            }
        }
    }

    // libmxm_engine.so = Rust (staticlib) + adapter C++ (ADR-0003).
    externalNativeBuild {
        cmake {
            path = file("src/main/cpp/CMakeLists.txt")
        }
    }

    buildTypes {
        release {
            // Sementara memakai kunci debug supaya build rilis bisa dicoba.
            // Kunci pratinjau milik pemilik (GitHub secret) dipasang di M0
            // langkah 7 (ADR-0009 butir 7). Kunci tidak pernah ada di repo.
            signingConfig = signingConfigs.getByName("debug")
        }
    }
}

kotlin {
    compilerOptions {
        jvmTarget = org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_17
    }
}

flutter {
    source = "../.."
}
