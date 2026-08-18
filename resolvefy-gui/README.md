# Resolvefy

**PROJECT CREATED ENTIRELY WITH AI**

Resolvefy lets you convert a video (and its audio) to a codec compatible with DaVinci Resolve on Linux, using ffmpeg.

Output files are saved as MP4 with a `_resolve` suffix (e.g., `video_resolve.mp4`).

License: see LICENSE (GPL-3.0).

## Crates

- `resolvefy-core` — Core conversion logic
- `resolvefy-gui` — Libadwaita (GTK4) UI

## Dependencies

- Rust toolchain (rustup, stable toolchain)
- gcc, pkg-config (C compilation)
- ffmpeg >= 7.0 (with SVT-AV1 and libopus support)
- System libraries: GTK4, libadwaita

Examples (Debian/Ubuntu):

```sh
sudo apt update && sudo apt install -y \
    gcc pkg-config \
    ffmpeg \
    libgtk-4-dev libadwaita-1-dev
```

On Fedora:

```sh
sudo dnf install -y \
    gcc pkgconfig \
    ffmpeg \
    gtk4-devel libadwaita-devel
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
cargo run --release -p resolvefy-gui
```

## Notes

- Encoding options (CRF/CBR mode, values) are in an advanced collapsible section.
- Ensure the ffmpeg on your system includes the SVT-AV1 encoder and libopus if you need AV1/Opus encoding. Some distributions do not include SVT-AV1 by default; it may require manual compilation or alternative repositories.
