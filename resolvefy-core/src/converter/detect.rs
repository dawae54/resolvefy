//! Input file detection via `ffprobe`.
//!
//! Runs `ffprobe` to extract codec information and duration from a video file,
//! and determines whether re-encoding is needed.

use std::path::Path;
use std::process::Command;

use serde::Deserialize;

use super::InputInfo;

#[derive(Debug, Deserialize)]
struct ProbeStream {
    codec_type: String,
    codec_name: String,
}

#[derive(Debug, Deserialize)]
struct ProbeFormat {
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProbeOutput {
    streams: Vec<ProbeStream>,
    format: ProbeFormat,
}

/// Runs `ffprobe` on the given path and returns the parsed JSON output.
fn run_ffprobe(path: &Path) -> Result<ProbeOutput, String> {
    let output = Command::new("ffprobe")
        .args([
            "-v", "quiet",
            "-print_format", "json",
            "-show_streams",
            "-show_format",
            path.to_str().unwrap_or_default(),
        ])
        .output()
        .map_err(|e| format!("failed to run ffprobe: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("ffprobe failed: {stderr}"));
    }

    let stdout = String::from_utf8(output.stdout)
        .map_err(|e| format!("ffprobe output not valid utf-8: {e}"))?;

    serde_json::from_str(&stdout)
        .map_err(|e| format!("failed to parse ffprobe output: {e}"))
}

/// Detects video and audio codecs and duration for the given input file.
///
/// Returns an [`InputInfo`] describing the streams and whether each is already
/// in the target format (AV1 for video, Opus for audio).
///
/// # Errors
///
/// Returns `Err` if `ffprobe` cannot be executed or the output cannot be parsed.
pub fn detect_input(path: &Path) -> Result<InputInfo, String> {
    let probe = run_ffprobe(path)?;

    let mut video_codec = String::new();
    let mut audio_codec = String::new();

    for stream in &probe.streams {
        match stream.codec_type.as_str() {
            "video" if video_codec.is_empty() => {
                video_codec = stream.codec_name.clone();
            }
            "audio" if audio_codec.is_empty() => {
                audio_codec = stream.codec_name.clone();
            }
            _ => {}
        }
    }

    let duration_secs = probe
        .format
        .duration
        .as_deref()
        .and_then(|d| d.parse::<f64>().ok())
        .unwrap_or(0.0);

    Ok(InputInfo {
        is_video_av1: video_codec == "av1",
        is_audio_opus: audio_codec == "opus",
        video_codec,
        audio_codec,
        duration_secs,
    })
}
