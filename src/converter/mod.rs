pub mod audio;
pub mod convert;
pub mod detect;
pub mod video;

use std::path::{Path, PathBuf};

pub use convert::convert;
pub use detect::detect_input;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncodeMode {
    CRF,
    CBR,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Container {
    MKV,
    MP4,
}

#[derive(Debug, Clone)]
pub struct InputInfo {
    pub video_codec: String,
    pub audio_codec: String,
    pub duration_secs: f64,
    pub is_video_av1: bool,
    pub is_audio_opus: bool,
}

pub struct EncodeConfig {
    pub mode: EncodeMode,
    pub crf_value: u32,
    pub bitrate_kbps: u32,
}

pub fn format_duration(secs: f64) -> String {
    let h = secs as u64 / 3600;
    let m = (secs as u64 % 3600) / 60;
    let s = secs as u64 % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

pub fn output_extension(container: Container) -> &'static str {
    match container {
        Container::MKV => "mkv",
        Container::MP4 => "mp4",
    }
}

pub fn default_output_name(input: &Path, container: Container) -> PathBuf {
    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
    let ext = output_extension(container);
    input.with_file_name(format!("{stem}.{ext}"))
}
