use std::path::PathBuf;

use ffmpeg_next as ffmpeg;

use super::{InputInfo, EncodeConfig};
use super::video::VideoTranscoder;
use super::audio::AudioTranscoder;

pub fn convert(
    input: PathBuf,
    output: PathBuf,
    config: EncodeConfig,
    info: &InputInfo,
    progress_cb: impl Fn(f64, String),
) -> Result<(), String> {
    ffmpeg::init().map_err(|e| format!("ffmpeg init: {e}"))?;

    let mut ictx = ffmpeg::format::input(&input).map_err(|e| format!("open input: {e}"))?;

    let mut octx =
        ffmpeg::format::output(&output).map_err(|e| format!("open output: {e}"))?;

    let nb_streams = ictx.nb_streams() as usize;
    let mut stream_mapping: Vec<isize> = vec![-1; nb_streams];
    let mut ost_index: usize = 0;

    let mut video_transcoder: Option<VideoTranscoder> = None;
    let mut audio_transcoder: Option<AudioTranscoder> = None;

    for (ist_index, ist) in ictx.streams().enumerate() {
        let medium = ist.parameters().medium();
        let codec_id = ist.parameters().id();

        match medium {
            ffmpeg::media::Type::Video => {
                if codec_id == ffmpeg::codec::Id::AV1 {
                    let mut ost = octx
                        .add_stream(ffmpeg::encoder::find(ffmpeg::codec::Id::None))
                        .map_err(|e| format!("add stream: {e}"))?;
                    ost.set_parameters(ist.parameters());
                    ost.set_time_base(ist.time_base());
                    unsafe {
                        (*ost.parameters().as_mut_ptr()).codec_tag = 0;
                    }
                    stream_mapping[ist_index] = ost_index as isize;
                } else {
                    let transcoder =
                        VideoTranscoder::new(&ist, &mut octx, ost_index, &config)
                            .map_err(|e| format!("init video transcoder: {e}"))?;
                    stream_mapping[ist_index] = ost_index as isize;
                    video_transcoder = Some(transcoder);
                }
                ost_index += 1;
            }
            ffmpeg::media::Type::Audio => {
                if codec_id == ffmpeg::codec::Id::OPUS {
                    let mut ost = octx
                        .add_stream(ffmpeg::encoder::find(ffmpeg::codec::Id::None))
                        .map_err(|e| format!("add stream: {e}"))?;
                    ost.set_parameters(ist.parameters());
                    ost.set_time_base(ist.time_base());
                    unsafe {
                        (*ost.parameters().as_mut_ptr()).codec_tag = 0;
                    }
                    stream_mapping[ist_index] = ost_index as isize;
                } else {
                    let transcoder = AudioTranscoder::new(&ist, &mut octx, ost_index)
                        .map_err(|e| format!("init audio transcoder: {e}"))?;
                    stream_mapping[ist_index] = ost_index as isize;
                    audio_transcoder = Some(transcoder);
                }
                ost_index += 1;
            }
            _ => {
                stream_mapping[ist_index] = -1;
            }
        }
    }

    octx.set_metadata(ictx.metadata().to_owned());
    octx.write_header()
        .map_err(|e| format!("write header: {e}"))?;

    let total_duration = info.duration_secs;

    for (stream, packet) in ictx.packets() {
        let ist_index = stream.index();
        let ost_index_val = stream_mapping[ist_index];
        if ost_index_val < 0 {
            continue;
        }

        let medium = stream.parameters().medium();
        let tb = f64::from(stream.time_base());
        let pts_val = packet.pts().or_else(|| packet.dts());

        match medium {
            ffmpeg::media::Type::Video if video_transcoder.is_some() => {
                if let Some(ref mut vt) = video_transcoder {
                    vt.send_packet(&packet);
                    vt.receive_and_write(&mut octx, &progress_cb, total_duration);
                }
            }
            ffmpeg::media::Type::Audio if audio_transcoder.is_some() => {
                if let Some(ref mut at) = audio_transcoder {
                    at.send_packet(&packet);
                    at.receive_and_write(&mut octx);
                }
            }
            _ => {
                let mut pkt = packet;
                let in_tb = stream.time_base();
                let out_tb = octx
                    .stream(ost_index_val as usize)
                    .map(|s| s.time_base())
                    .unwrap_or(ffmpeg::Rational(1, 90000));
                pkt.rescale_ts(in_tb, out_tb);
                pkt.set_position(-1);
                pkt.set_stream(ost_index_val as usize);
                let _ = pkt.write_interleaved(&mut octx);
            }
        }

        if total_duration > 0.0 {
            if let Some(ts) = pts_val {
                let current = ts as f64 * tb;
                let pct = (current / total_duration * 100.0).min(100.0);
                progress_cb(pct, super::format_duration(current));
            }
        }
    }

    if let Some(ref mut vt) = video_transcoder {
        vt.flush(&mut octx);
    }
    if let Some(ref mut at) = audio_transcoder {
        at.flush(&mut octx);
    }

    octx.write_trailer()
        .map_err(|e| format!("write trailer: {e}"))?;

    progress_cb(100.0, "done".into());
    Ok(())
}
