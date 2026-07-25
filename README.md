# Resolvefy

**PROYECTO CREADO COMPLETAMENTE CON IA**

Resolvefy es un conversor de vídeo a AV1 escrito en Rust usando libadwaita. La interfaz y buena parte del código fueron generados con ayuda de inteligencia artificial.

Resumen rápido:
- Interfaz: libadwaita (GTK)
- Lógica: conversión AV1 (SVT‑AV1/Opus a través de ffmpeg)

Licencia: ver LICENSE (GPL-3.0). Nota: el campo "license" en Cargo.toml puede diferir.

Dependencias
------------
Dependencias de crates (en Cargo.toml):
- ffmpeg-next
- gtk4
- libadwaita
- rfd

Dependencias de sistema (necesarias para compilar y ejecutar):
- Rust toolchain (rustup, toolchain estable)
- pkg-config, build-essential (compilación C/C++)
- GTK4 y libadwaita (desarrollo) — p.ej. libgtk-4-dev, libadwaita-1-dev
- gir y GObject introspection: libgirepository1.0-dev (si tu distro lo requiere)
- ffmpeg (con soporte para SVT-AV1 y libopus) instalado en el sistema
- xdg-desktop-portal (para diálogos nativos usados por rfd)

Ejemplos (Debian/Ubuntu):

sudo apt update && sudo apt install -y build-essential pkg-config libgtk-4-dev libadwaita-1-dev libgirepository1.0-dev ffmpeg xdg-desktop-portal

En Fedora:

sudo dnf install -y gcc-c++ pkgconfig gtk4-devel libadwaita-devel ffmpeg xdg-desktop-portal

Compilar y ejecutar
--------------------
Instalar Rust (si no está):

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

Compilar en modo release:

cargo build --release

Ejecutar directamente con cargo (modo desarrollo):

cargo run --release

O ejecutar el binario compilado:

./target/release/resolvefy

Notas
-----
- Asegúrate de que ffmpeg en tu sistema incluya el encoder SVT-AV1 y libopus si necesitas codificación AV1/Opus. Algunas distribuciones no incluyen SVT-AV1 por defecto; podría requerir compilación manual o repositorios alternativos.
- La UI usa xdg portals para los diálogos de archivo (rfd). En entornos de escritorio sandboxed (Flatpak) puede requerir configuración adicional.

README generado/actualizado automáticamente y commiteado.
