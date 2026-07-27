use std::path::Path;

use ffmpeg_next as ffmpeg;

use super::InputInfo;

struct StreamInfo {
    video_codec: Option<String>,
    audio_codec: Option<String>,
    video_duration: Option<f64>,
}

fn extract_stream_info(stream: ffmpeg::format::stream::Stream) -> Option<StreamInfo> {
    let medium = stream.parameters().medium();
    let codec_id = stream.parameters().id();

    match medium {
        ffmpeg::media::Type::Video => {
            let duration = (stream.duration() > 0).then(|| {
                let tb = f64::from(stream.time_base());
                stream.duration() as f64 * tb
            });
            Some(StreamInfo {
                video_codec: Some(format!("{codec_id:?}")),
                audio_codec: None,
                video_duration: duration,
            })
        }
        ffmpeg::media::Type::Audio => Some(StreamInfo {
            video_codec: None,
            audio_codec: Some(format!("{codec_id:?}")),
            video_duration: None,
        }),
        _ => None,
    }
}

fn merge_stream_info(acc: &mut StreamInfo, info: StreamInfo) {
    if acc.video_codec.is_none() {
        acc.video_codec = info.video_codec;
    }
    if acc.video_duration.is_none() {
        acc.video_duration = info.video_duration;
    }
    if acc.audio_codec.is_none() {
        acc.audio_codec = info.audio_codec;
    }
}

fn calculate_duration(ictx: &ffmpeg::format::context::Input, video_duration: Option<f64>) -> f64 {
    let duration_raw = ictx.duration();
    let base_duration = if duration_raw > 0 && duration_raw != i64::MIN {
        duration_raw as f64 / ffmpeg::ffi::AV_TIME_BASE as f64
    } else {
        0.0
    };

    if base_duration > 0.0 {
        base_duration
    } else {
        video_duration.unwrap_or(0.0)
    }
}

pub fn detect_input(path: &Path) -> Result<InputInfo, String> {
    ffmpeg::init().map_err(|e| format!("ffmpeg init: {e}"))?;
    let ictx = ffmpeg::format::input(path).map_err(|e| format!("open input: {e}"))?;

    let initial = StreamInfo {
        video_codec: None,
        audio_codec: None,
        video_duration: None,
    };

    let stream_info = ictx
        .streams()
        .filter_map(extract_stream_info)
        .fold(initial, |mut acc, info| {
            merge_stream_info(&mut acc, info);
            acc
        });

    let duration_secs = calculate_duration(&ictx, stream_info.video_duration);

    let video_codec = stream_info.video_codec.unwrap_or_default();
    let audio_codec = stream_info.audio_codec.unwrap_or_default();

    Ok(InputInfo {
        is_video_av1: video_codec.contains("AV1"),
        is_audio_opus: audio_codec.contains("OPUS"),
        video_codec,
        audio_codec,
        duration_secs,
    })
}
