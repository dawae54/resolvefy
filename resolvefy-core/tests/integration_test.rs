use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use resolvefy_core::converter::{self, EncodeConfig, EncodeMode, InputInfo};

fn fixtures_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

#[test]
fn detect_h264_aac_input() {
    let path = fixtures_dir().join("h264_aac.mp4");
    let info = converter::detect_input(&path).expect("failed to detect input");

    assert!(!info.is_video_av1, "H264 should not be detected as AV1");
    assert!(!info.is_audio_opus, "AAC should not be detected as Opus");
    assert!(info.duration_secs > 0.0, "duration should be positive");
    assert!(!info.video_codec.is_empty(), "video codec should not be empty");
    assert!(!info.audio_codec.is_empty(), "audio codec should not be empty");
}

#[test]
fn detect_av1_opus_input() {
    let path = fixtures_dir().join("av1_opus.mp4");
    let info = converter::detect_input(&path).expect("failed to detect input");

    assert!(info.is_video_av1, "AV1 should be detected");
    assert!(info.is_audio_opus, "Opus should be detected");
    assert!(info.duration_secs > 0.0);
}

#[test]
fn detect_h264_opus_input() {
    let path = fixtures_dir().join("h264_opus.mp4");
    let info = converter::detect_input(&path).expect("failed to detect input");

    assert!(!info.is_video_av1);
    assert!(info.is_audio_opus);
}

#[test]
fn detect_av1_aac_input() {
    let path = fixtures_dir().join("av1_aac.mp4");
    let info = converter::detect_input(&path).expect("failed to detect input");

    assert!(info.is_video_av1);
    assert!(!info.is_audio_opus);
}

#[test]
fn detect_nonexistent_file() {
    let path = Path::new("/nonexistent/video.mp4");
    let result = converter::detect_input(path);
    assert!(result.is_err(), "should fail for nonexistent file");
}

#[test]
fn convert_h264_aac_to_resolve_format() {
    let input = fixtures_dir().join("h264_aac.mp4");
    let output = fixtures_dir().join("test_output_h264_aac.mp4");

    let info = converter::detect_input(&input).expect("failed to detect input");

    let config = EncodeConfig {
        mode: EncodeMode::CRF,
        crf_value: 30,
        bitrate_kbps: 0,
    };

    let progress_calls = Arc::new(Mutex::new(Vec::new()));
    let progress_calls_clone = progress_calls.clone();

    converter::convert(
        input.clone(),
        output.clone(),
        config,
        &info,
        move |pct, status| {
            progress_calls_clone.lock().unwrap().push((pct, status));
        },
    )
    .expect("conversion failed");

    assert!(output.exists(), "output file should exist");
    assert!(output.metadata().unwrap().len() > 0, "output should not be empty");

    let calls = progress_calls.lock().unwrap();
    assert!(!calls.is_empty(), "should have progress callbacks");
    assert_eq!(calls.last().unwrap().0, 100.0, "final progress should be 100%");

    std::fs::remove_file(&output).ok();
}

#[test]
fn convert_av1_opus_passthrough() {
    let input = fixtures_dir().join("av1_opus.mp4");
    let output = fixtures_dir().join("test_output_av1_opus.mp4");

    let info = converter::detect_input(&input).expect("failed to detect input");
    assert!(info.is_video_av1);
    assert!(info.is_audio_opus);

    let config = EncodeConfig {
        mode: EncodeMode::CRF,
        crf_value: 30,
        bitrate_kbps: 0,
    };

    converter::convert(input, output.clone(), config, &info, |_, _| {})
        .expect("conversion failed");

    assert!(output.exists());
    assert!(output.metadata().unwrap().len() > 0);

    std::fs::remove_file(&output).ok();
}

#[test]
fn convert_with_cbr_mode() {
    let input = fixtures_dir().join("short_video.mp4");
    let output = fixtures_dir().join("test_output_cbr.mp4");

    let info = converter::detect_input(&input).expect("failed to detect input");

    let config = EncodeConfig {
        mode: EncodeMode::CBR,
        crf_value: 0,
        bitrate_kbps: 2000,
    };

    converter::convert(input, output.clone(), config, &info, |_, _| {})
        .expect("conversion failed");

    assert!(output.exists());
    assert!(output.metadata().unwrap().len() > 0);

    std::fs::remove_file(&output).ok();
}

#[test]
fn convert_nonexistent_input() {
    let input = Path::new("/nonexistent/video.mp4");
    let output = fixtures_dir().join("test_output_nonexistent.mp4");

    let config = EncodeConfig {
        mode: EncodeMode::CRF,
        crf_value: 30,
        bitrate_kbps: 0,
    };

    let info = InputInfo {
        video_codec: "H264".to_string(),
        audio_codec: "AAC".to_string(),
        duration_secs: 10.0,
        is_video_av1: false,
        is_audio_opus: false,
    };

    let result = converter::convert(input.to_path_buf(), output, config, &info, |_, _| {});
    assert!(result.is_err(), "should fail for nonexistent input");
}

#[test]
fn convert_progress_callbacks_are_ordered() {
    let input = fixtures_dir().join("short_video.mp4");
    let output = fixtures_dir().join("test_output_progress.mp4");

    let info = converter::detect_input(&input).expect("failed to detect input");

    let config = EncodeConfig {
        mode: EncodeMode::CRF,
        crf_value: 30,
        bitrate_kbps: 0,
    };

    let progress_values = Arc::new(Mutex::new(Vec::new()));
    let progress_values_clone = progress_values.clone();

    converter::convert(input, output.clone(), config, &info, move |pct, _| {
        progress_values_clone.lock().unwrap().push(pct);
    })
    .expect("conversion failed");

    let values = progress_values.lock().unwrap();
    assert!(values.len() > 1, "should have multiple progress updates");

    for window in values.windows(2) {
        assert!(
            window[0] <= window[1],
            "progress should be non-decreasing: {} > {}",
            window[0],
            window[1]
        );
    }

    assert_eq!(*values.last().unwrap(), 100.0);

    std::fs::remove_file(&output).ok();
}

#[test]
fn detect_input_duration_is_reasonable() {
    let path = fixtures_dir().join("h264_aac.mp4");
    let info = converter::detect_input(&path).expect("failed to detect input");

    assert!(
        info.duration_secs >= 1.5 && info.duration_secs <= 3.0,
        "duration should be around 2 seconds, got {}",
        info.duration_secs
    );
}

#[test]
fn detect_input_codecs_are_populated() {
    let path = fixtures_dir().join("h264_aac.mp4");
    let info = converter::detect_input(&path).expect("failed to detect input");

    assert!(
        info.video_codec.contains("H264") || info.video_codec.contains("h264"),
        "video codec should contain H264, got: {}",
        info.video_codec
    );
    assert!(
        info.audio_codec.contains("AAC") || info.audio_codec.contains("aac"),
        "audio codec should contain AAC, got: {}",
        info.audio_codec
    );
}

#[test]
fn convert_yuv444p_to_yuv420p() {
    let input = fixtures_dir().join("h264_aac_yuv444p.mp4");
    let output = fixtures_dir().join("test_output_yuv444p.mp4");

    let info = converter::detect_input(&input).expect("failed to detect input");

    let config = EncodeConfig {
        mode: EncodeMode::CRF,
        crf_value: 30,
        bitrate_kbps: 0,
    };

    converter::convert(input, output.clone(), config, &info, |_, _| {})
        .expect("conversion failed");

    assert!(output.exists());
    assert!(output.metadata().unwrap().len() > 0);

    let out_info = converter::detect_input(&output).expect("failed to detect output");
    assert_eq!(out_info.video_codec, "av1");

    std::fs::remove_file(&output).ok();
}

#[test]
fn convert_yuv422p_to_yuv420p() {
    let input = fixtures_dir().join("h264_aac_yuv422p.mp4");
    let output = fixtures_dir().join("test_output_yuv422p.mp4");

    let info = converter::detect_input(&input).expect("failed to detect input");

    let config = EncodeConfig {
        mode: EncodeMode::CRF,
        crf_value: 30,
        bitrate_kbps: 0,
    };

    converter::convert(input, output.clone(), config, &info, |_, _| {})
        .expect("conversion failed");

    assert!(output.exists());
    assert!(output.metadata().unwrap().len() > 0);

    let out_info = converter::detect_input(&output).expect("failed to detect output");
    assert_eq!(out_info.video_codec, "av1");

    std::fs::remove_file(&output).ok();
}