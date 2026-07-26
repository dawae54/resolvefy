use std::path::Path;

use ffmpeg_next as ffmpeg;

use super::InputInfo;

pub fn detect_input(path: &Path) -> Result<InputInfo, String> {
    ffmpeg::init().map_err(|e| format!("ffmpeg init: {e}"))?;

    let ictx = ffmpeg::format::input(path).map_err(|e| format!("open input: {e}"))?;

    let mut video_codec = String::new();
    let mut audio_codec = String::new();
    let mut video_stream_duration: Option<f64> = None;

    for stream in ictx.streams() {
        let codec_id = stream.parameters().id();
        let medium = stream.parameters().medium();

        match medium {
            ffmpeg::media::Type::Video if video_codec.is_empty() => {
                video_codec = format!("{codec_id:?}");
                if stream.duration() > 0 {
                    let tb = f64::from(stream.time_base());
                    video_stream_duration = Some(stream.duration() as f64 * tb);
                }
            }
            ffmpeg::media::Type::Audio if audio_codec.is_empty() => {
                audio_codec = format!("{codec_id:?}");
            }
            _ => {}
        }
    }

    let duration_raw = ictx.duration();
    let mut duration_secs = if duration_raw > 0 && duration_raw != i64::MIN {
        duration_raw as f64 / ffmpeg::ffi::AV_TIME_BASE as f64
    } else {
        0.0
    };

    if duration_secs <= 0.0 {
        if let Some(d) = video_stream_duration {
            duration_secs = d;
        }
    }

    let is_video_av1 = video_codec.contains("AV1");
    let is_audio_opus = audio_codec.contains("OPUS");

    Ok(InputInfo {
        video_codec,
        audio_codec,
        duration_secs,
        is_video_av1,
        is_audio_opus,
    })
}
