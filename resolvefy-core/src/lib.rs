//! Core library for Resolvefy: video codec detection and conversion via ffmpeg.
//!
//! This crate provides the shared logic used by both frontends (Slint, GTK):
//!
//! - Input codec detection via `ffprobe`.
//! - Video conversion to AV1 (SVT-AV1) and audio to Opus with real-time progress.
//! - Application state shared between the UI and conversion logic.

pub mod converter;
pub mod state;

pub use state::{AppState, ProgressState};
