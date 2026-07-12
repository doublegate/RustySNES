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
        versionName = "1.15.0"
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
            "ANDROID_NDK_HOME must be set to build the native rustysnes-mobile/rustysnes-android libraries"
        )
    environment("ANDROID_NDK_HOME", ndkHome)
    workingDir = rootProject.projectDir.parentFile
    val targetArgs = cargoAbis.keys.flatMap { listOf("-t", it) }
    commandLine(
        listOf("cargo", "ndk") + targetArgs +
            listOf("build", "-p", "rustysnes-mobile", "-p", "rustysnes-android")
    )
}

// One `Copy` task per ABI (not a single task looping `from`/`into`) -- `Copy` only honors the
// LAST `into()` when called repeatedly, which silently merged both ABIs' `.so`s into one
// destination and tripped Gradle's duplicate-entry guard (found by actually running this).
val copyCargoLibTasks = cargoAbis.map { (abi, triple) ->
    tasks.register<Copy>("copyCargoLibs${abi.replace("-", "")}") {
        dependsOn("cargoNdkBuild")
        from(rootProject.projectDir.parentFile.resolve("target/$triple/debug")) {
            include("librustysnes_mobile.so", "librustysnes_android.so")
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

android.sourceSets.getByName("main").kotlin.srcDir("build/generated/uniffi/uniffi")

tasks.named("preBuild") {
    dependsOn("copyCargoLibs", "uniffiBindgen")
}

dependencies {
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
    implementation("net.java.dev.jna:jna:5.15.0@aar")
}
