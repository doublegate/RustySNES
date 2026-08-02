plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose") version "2.0.20"
}

android {
    namespace = "com.doublegate.rustysnes"
    // 35, not 34 (the AVD's own API level) -- `androidx.core:core-ktx:1.15.0` requires
    // compiling against 35+ (found by actually building; `compileSdk` is independent of
    // `targetSdk`/`minSdk`/the AVD's runtime API, so this doesn't change device compatibility).
    compileSdk = 35

    defaultConfig {
        applicationId = "com.doublegate.rustysnes"
        // NDK r29's own minimum supported API level.
        minSdk = 21
        targetSdk = 34
        versionCode = 1
        versionName = "1.18.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }

    buildFeatures {
        compose = true
    }

    // The two `.so`s (`librustysnes_mobile.so`, the UniFFI-bridged emulation core; and
    // `librustysnes_android.so`, the wgpu-on-Surface renderer) are built via `cargo ndk` by the
    // `cargoNdkBuild` task below, which runs before every `assemble*`/`preBuild` -- they are NOT
    // checked into the repo (matching the project's "never commit prebuilt binaries" convention),
    // so `jniLibs.srcDirs` points at the build-time output directory, not a source-controlled one.
    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
        // The instrumented UniFFI smoke test needs the same generated bindings the app uses --
        // `androidTest` compiles as its own variant and does not inherit `main`'s generated
        // sources automatically.
        getByName("androidTest") {
            kotlin.srcDirs("src/androidTest/kotlin")
            // The instrumented smoke test loads a REAL cart, so it needs one packaged with it.
            // Copied in by `copyTestRom` below rather than checked in twice.
            assets.srcDirs("src/androidTest/assets")
        }
    }
}

// Builds both native crates for every ABI Gradle is about to package, then copies the resulting
// `.so`s into `src/main/jniLibs/<abi>/` where the Android Gradle Plugin's own packaging step
// picks them up automatically (no manual `abiFilters`/packaging config needed beyond this).
// `ANDROID_NDK_HOME` must be set in the environment (matches `cargo-ndk`'s own requirement) --
// this task fails loudly with a clear message rather than silently skipping if it isn't.
val cargoAbis = mapOf(
    "arm64-v8a" to "aarch64-linux-android",
    "x86_64" to "x86_64-linux-android",
)

tasks.register<Exec>("cargoNdkBuild") {
    val ndkHome = System.getenv("ANDROID_NDK_HOME")
        ?: throw GradleException(
            "ANDROID_NDK_HOME must be set to build the native rustysnes-mobile/rustysnes-android/rustysnes-monetization libraries"
        )
    environment("ANDROID_NDK_HOME", ndkHome)
    workingDir = rootProject.projectDir.parentFile
    val targetArgs = cargoAbis.keys.flatMap { listOf("-t", it) }
    commandLine(
        listOf("cargo", "ndk") + targetArgs +
            listOf("build", "-p", "rustysnes-mobile", "-p", "rustysnes-android", "-p", "rustysnes-monetization")
    )
}

// One `Copy` task per ABI (not a single task looping `from`/`into`) -- `Copy` only honors the
// LAST `into()` when called repeatedly, which silently merged both ABIs' `.so`s into one
// destination and tripped Gradle's duplicate-entry guard (found by actually running this).
val copyCargoLibTasks = cargoAbis.map { (abi, triple) ->
    tasks.register<Copy>("copyCargoLibs${abi.replace("-", "")}") {
        dependsOn("cargoNdkBuild")
        from(rootProject.projectDir.parentFile.resolve("target/$triple/debug")) {
            include("librustysnes_mobile.so", "librustysnes_android.so", "librustysnes_monetization.so")
        }
        into(project.projectDir.resolve("src/main/jniLibs/$abi"))
    }
}
tasks.register("copyCargoLibs") {
    dependsOn(copyCargoLibTasks)
}

// The UniFFI-generated Kotlin bindings for `rustysnes-mobile`'s emulation-core surface --
// regenerated from the just-built `.so` on every build rather than checked in, so the bindings
// can never drift from the Rust source they're generated from (see `docs/mobile-readiness.md`).
tasks.register<Exec>("uniffiBindgen") {
    dependsOn("cargoNdkBuild")
    workingDir = rootProject.projectDir.parentFile
    val soPath = rootProject.projectDir.parentFile
        .resolve("target/x86_64-linux-android/debug/librustysnes_mobile.so")
    val outDir = project.projectDir.resolve("build/generated/uniffi")
    commandLine(
        "cargo", "run", "-p", "rustysnes-mobile", "--features", "bindgen", "--bin", "uniffi-bindgen",
        "--", "generate", "--library", soPath.absolutePath, "--language", "kotlin",
        "--out-dir", outDir.absolutePath, "--no-format",
    )
}

// Same shape as `uniffiBindgen` above, for `rustysnes-monetization`'s own separate UniFFI
// library -- a distinct crate/`.so`/namespace, so it needs its own bindgen invocation and output
// directory (`v1.18.0 "Dormant"`).
tasks.register<Exec>("uniffiBindgenMonetization") {
    dependsOn("cargoNdkBuild")
    workingDir = rootProject.projectDir.parentFile
    val soPath = rootProject.projectDir.parentFile
        .resolve("target/x86_64-linux-android/debug/librustysnes_monetization.so")
    val outDir = project.projectDir.resolve("build/generated/uniffi-monetization")
    commandLine(
        "cargo", "run", "-p", "rustysnes-monetization", "--features", "bindgen", "--bin", "uniffi-bindgen",
        "--", "generate", "--library", soPath.absolutePath, "--language", "kotlin",
        "--out-dir", outDir.absolutePath, "--no-format",
    )
}

android.sourceSets.getByName("main").kotlin.srcDir("build/generated/uniffi/uniffi")
android.sourceSets.getByName("main").kotlin.srcDir("build/generated/uniffi-monetization/uniffi")

// AccuracySNES's HiROM image (64 KB) as an instrumented-test asset.
//
// This project's own cart, dual-licensed with the repo, so unlike every commercial ROM it can be
// packaged into a test APK. It is what turns the smoke test from "the bindings load" into "a real
// cart boots on a device" -- `docs/mobile-readiness.md` records that no ROM had ever actually
// booted on a device or simulator, and an emulator bridge test with no ROM cannot fix that.
//
// The HiROM variant, not the 256 KB LoROM one, because the test only needs a cart that runs and
// this is the smallest of the four the generator emits.
val copyTestRom = tasks.register<Copy>("copyTestRom") {
    from(rootProject.projectDir.parentFile.resolve("tests/roms/AccuracySNES/build")) {
        include("accuracysnes-hirom.sfc")
    }
    into(project.projectDir.resolve("src/androidTest/assets"))
}

tasks.named("preBuild") {
    dependsOn("copyCargoLibs", "uniffiBindgen", "uniffiBindgenMonetization", copyTestRom)
}

dependencies {
    // The instrumented UniFFI smoke test (`src/androidTest`). It proves the generated bindings
    // LOAD and CALL on a device, which a build cannot: `assembleDebug` already proves they
    // compile, because `MainActivity` calls `MobileCore` directly.
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test:runner:1.6.2")

    implementation("androidx.core:core-ktx:1.15.0")
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.8.7")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")
    implementation(platform("androidx.compose:compose-bom:2024.12.01"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui-tooling-preview")
    // The AAR classifier is required on Android -- the plain `net.java.dev.jna:jna` jar (what
    // UniFFI's Kotlin bindings assume on a desktop JVM) does not bundle Android's native
    // `libjnidispatch.so`.
    //
    // 5.17.0, not 5.15.0, because of the 16 KB page-alignment requirement -- and the version was
    // chosen by measuring the published AARs, per ABI, rather than from a changelog:
    //
    //   5.15.0  arm64-v8a 0x10000  x86_64 0x1000  armeabi-v7a 0x1000  x86 0x1000
    //   5.16.0+ arm64-v8a 0x4000   x86_64 0x4000  armeabi-v7a 0x4000  x86 0x4000
    //
    // So 5.15.0 satisfies the requirement on arm64 (64 KB is a multiple of 16 KB) and violates it
    // on every other ABI -- which is why checking one ABI and generalising gave the wrong answer
    // the first time. 5.16.0 is the first release that fixes it; 5.17.0 is taken as the nearest
    // settled patch line after that change.
    implementation("net.java.dev.jna:jna:5.17.0@aar")
}
