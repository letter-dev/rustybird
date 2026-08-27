# RustyBird

An open-source Flappy Bird remake written in Rust.

## Build from source

```bash
git clone https://github.com/letter-dev/RustyBird.git
cd RustyBird
cargo run --release
```

Requires [Rust](https://rustup.rs/).

### Android

Prerequisites: Android SDK (build-tools, platform), JDK, NDK r27+.

```powershell
powershell -ExecutionPolicy Bypass -File android\build_apk.ps1
```

Outputs a universal APK (arm64-v8a, armeabi-v7a, x86, x86_64). Edit linker paths in `.cargo/config.toml` if your NDK lives elsewhere.

## Download

[Releases](../../releases) — `RustyBird-windows.zip` (run `RustyBird.exe`) or `RustyBird-universal.apk` (install on Android).

## Tech

- [Rust](https://www.rust-lang.org/)
- [macroquad](https://github.com/not-fl3/macroquad) — game engine
