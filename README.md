# Resolvefy

**PROJECT CREATED ENTIRELY WITH AI**

Resolvefy lets you convert a video (and its audio) to a codec compatible with Davinci Resolve on Linux, using ffmpeg.

License: see LICENSE (GPL-3.0).

## Dependencies

- Rust toolchain (rustup, stable toolchain)
- pkg-config, build-essential (C/C++ compilation)
- GTK4 and libadwaita development packages — e.g., libgtk-4-dev, libadwaita-1-dev
- gir and GObject Introspection: libgirepository1.0-dev (if required by your distribution)
- ffmpeg (with SVT-AV1 and libopus support) installed on the system
- xdg-desktop-portal (for native dialogs used by rfd)

Examples (Debian/Ubuntu):

sudo apt update && sudo apt install -y build-essential pkg-config libgtk-4-dev libadwaita-1-dev libgirepository1.0-dev ffmpeg xdg-desktop-portal

On Fedora:
```sh
sudo dnf install -y gcc-c++ pkgconfig gtk4-devel libadwaita-devel ffmpeg xdg-desktop-portal
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

Run with cargo (development):

```sh
cargo run --release
```

Or run the compiled binary:

```sh
./target/release/resolvefy
```

## Notes

- Ensure the ffmpeg on your system includes the SVT-AV1 encoder and libopus if you need AV1/Opus encoding. Some distributions do not include SVT-AV1 by default; it may require manual compilation or alternative repositories.
- The UI uses xdg portals for file dialogs (rfd). In sandboxed desktop environments (e.g., Flatpak) additional configuration may be required.

README generated/updated automatically and committed.
