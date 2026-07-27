pub mod convert;
pub mod detect;

use std::path::{Path, PathBuf};

pub use convert::convert;
pub use detect::detect_input;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[allow(clippy::upper_case_acronyms)]
pub enum EncodeMode {
    #[default]
    CRF,
    CBR,
}

pub const OUTPUT_EXTENSION: &str = "mp4";

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
    let (hours, remaining) = ((secs as u64) / 3600, (secs as u64) % 3600);
    let (minutes, seconds) = (remaining / 60, remaining % 60);
    format!("{hours:02}:{minutes:02}:{seconds:02}")
}

pub fn default_output_name(input: &Path) -> PathBuf {
    let stem = input.file_stem().unwrap_or_default().to_string_lossy();
    input.with_file_name(format!("{stem}_resolve.{OUTPUT_EXTENSION}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_duration_zero() {
        assert_eq!(format_duration(0.0), "00:00:00");
    }

    #[test]
    fn format_duration_seconds_only() {
        assert_eq!(format_duration(45.0), "00:00:45");
    }

    #[test]
    fn format_duration_minutes_and_seconds() {
        assert_eq!(format_duration(125.0), "00:02:05");
    }

    #[test]
    fn format_duration_hours_minutes_seconds() {
        assert_eq!(format_duration(3661.0), "01:01:01");
    }

    #[test]
    fn format_duration_exact_hour() {
        assert_eq!(format_duration(3600.0), "01:00:00");
    }

    #[test]
    fn default_output_name_basic() {
        let input = Path::new("/videos/test.mp4");
        let output = default_output_name(input);
        assert_eq!(output, PathBuf::from("/videos/test_resolve.mp4"));
    }

    #[test]
    fn default_output_name_no_extension() {
        let input = Path::new("/videos/test");
        let output = default_output_name(input);
        assert_eq!(output, PathBuf::from("/videos/test_resolve.mp4"));
    }

    #[test]
    fn default_output_name_nested_path() {
        let input = Path::new("/deep/nested/path/video.avi");
        let output = default_output_name(input);
        assert_eq!(output, PathBuf::from("/deep/nested/path/video_resolve.mp4"));
    }

    #[test]
    fn encode_mode_default_is_crf() {
        let mode = EncodeMode::default();
        assert_eq!(mode, EncodeMode::CRF);
    }

    #[test]
    fn encode_mode_equality() {
        assert_eq!(EncodeMode::CRF, EncodeMode::CRF);
        assert_eq!(EncodeMode::CBR, EncodeMode::CBR);
        assert_ne!(EncodeMode::CRF, EncodeMode::CBR);
    }

    #[test]
    fn input_info_creation() {
        let info = InputInfo {
            video_codec: "H264".to_string(),
            audio_codec: "AAC".to_string(),
            duration_secs: 120.5,
            is_video_av1: false,
            is_audio_opus: false,
        };
        assert_eq!(info.video_codec, "H264");
        assert_eq!(info.audio_codec, "AAC");
        assert!((info.duration_secs - 120.5).abs() < f64::EPSILON);
        assert!(!info.is_video_av1);
        assert!(!info.is_audio_opus);
    }

    #[test]
    fn input_info_av1_opus() {
        let info = InputInfo {
            video_codec: "AV1".to_string(),
            audio_codec: "OPUS".to_string(),
            duration_secs: 60.0,
            is_video_av1: true,
            is_audio_opus: true,
        };
        assert!(info.is_video_av1);
        assert!(info.is_audio_opus);
    }
}
