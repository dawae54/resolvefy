# Resolvefy

**PROJECT CREATED WITH AI**

Resolvefy lets you convert a video (and its audio) to a codec compatible with DaVinci Resolve on Linux, using ffmpeg.

Output files are saved as MP4 with a `_resolve` suffix (e.g., `video_resolve.mp4`).

License: see LICENSE (GPL-3.0).

## Crates

- `resolvefy-core` — Core conversion logic
- `resolvefy-slint` — Slint UI

## Dependencies

- Rust toolchain (rustup, stable toolchain)
- gcc, pkg-config (C compilation)
- ffmpeg >= 7.0 (with SVT-AV1 and libopus support)
- System libraries: fontconfig, xkbcommon, X11, EGL, GL (for the Slint UI)

Examples (Debian/Ubuntu):

```sh
sudo apt update && sudo apt install -y \
    gcc pkg-config \
    ffmpeg \
    libfontconfig-dev libxkbcommon-dev \
    libx11-dev libegl1-mesa-dev libgl1-mesa-dev
```

On Fedora:

```sh
sudo dnf install -y \
    gcc pkgconfig \
    ffmpeg \
    fontconfig-devel libxkbcommon-devel \
    libX11-devel mesa-libEGL-devel mesa-libGL-devel
```

## Build and run

Install Rust (if not installed):

```sh
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

Build in release mode:

```sh
cargo build --release
```

Run:

```sh
cargo run --release -p resolvefy-slint
```

## Notes

- Encoding options (CRF/CBR mode, values) are in an advanced collapsible section.
- Ensure the ffmpeg on your system includes the SVT-AV1 encoder and libopus if you need AV1/Opus encoding. Some distributions do not include SVT-AV1 by default; it may require manual compilation or alternative repositories.
