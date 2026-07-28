//! Video conversion pipeline using `ffmpeg`.
//!
//! Spawns an `ffmpeg` process that re-encodes video to AV1 (SVT-AV1) and audio
//! to Opus, or stream-copies when the codecs are already compatible. Reports
//! progress via a callback that receives the percentage (0.0–100.0) and current
//! timestamp.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::io::{BufRead, BufReader};

use super::{EncodeConfig, EncodeMode, InputInfo};

/// Converts a video file to AV1/Opus in an MP4 container.
///
/// If the input video is already AV1 or the audio is already Opus, the
/// corresponding stream is copied without re-encoding.
///
/// # Arguments
///
/// * `input` — Path to the source video file.
/// * `output` — Path for the resulting MP4 file.
/// * `config` — Encoding parameters (CRF/CBR mode, values).
/// * `info` — Detected codec information for the input file.
/// * `progress_cb` — Callback invoked with `(percentage, timestamp)` as
///   conversion progresses. `percentage` ranges from 0.0 to 100.0.
///
/// # Errors
///
/// Returns `Err` if `ffmpeg` cannot be spawned, fails during conversion, or
/// exits with a non-zero status.
pub fn convert(
    input: PathBuf,
    output: PathBuf,
    config: EncodeConfig,
    info: &InputInfo,
    progress_cb: impl Fn(f64, String),
) -> Result<(), String> {
    let mut args = vec![
        "-y".to_string(),
        "-i".to_string(),
        input.to_str().unwrap_or_default().to_string(),
    ];

    if !info.is_video_av1 {
        args.extend([
            "-c:v".to_string(),
            "libsvtav1".to_string(),
            "-pix_fmt".to_string(),
            "yuv420p".to_string(),
        ]);
        match config.mode {
            EncodeMode::CRF => {
                args.extend(["-crf".to_string(), config.crf_value.to_string()]);
            }
            EncodeMode::CBR => {
                args.extend(["-b:v".to_string(), format!("{}k", config.bitrate_kbps)]);
            }
        }
    } else {
        args.extend(["-c:v".to_string(), "copy".to_string()]);
    }

    if !info.is_audio_opus {
        args.extend([
            "-c:a".to_string(),
            "libopus".to_string(),
        ]);
    } else {
        args.extend(["-c:a".to_string(), "copy".to_string()]);
    }

    args.extend([
        "-progress".to_string(),
        "pipe:1".to_string(),
        output.to_str().unwrap_or_default().to_string(),
    ]);

    let mut child = Command::new("ffmpeg")
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to spawn ffmpeg: {e}"))?;

    let total_duration = info.duration_secs;

    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            let line = line.map_err(|e| format!("read ffmpeg output: {e}"))?;
            if let Some(value) = line.strip_prefix("out_time_us=") {
                if let Ok(micros) = value.parse::<i64>() {
                    let current = micros as f64 / 1_000_000.0;
                    if total_duration > 0.0 {
                        let pct = (current / total_duration * 100.0).min(100.0);
                        let (h, rem) = ((current as u64) / 3600, (current as u64) % 3600);
                        let (m, s) = (rem / 60, rem % 60);
                        progress_cb(pct, format!("{h:02}:{m:02}:{s:02}"));
                    }
                }
            }
        }
    }

    let status = child
        .wait()
        .map_err(|e| format!("failed to wait for ffmpeg: {e}"))?;

    if !status.success() {
        return Err(format!("ffmpeg exited with status {status}"));
    }

    progress_cb(100.0, "done".into());
    Ok(())
}
