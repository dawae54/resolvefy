use std::path::PathBuf;

use ffmpeg_next as ffmpeg;

use super::{EncodeConfig, InputInfo};
use super::audio::AudioTranscoder;
use super::video::VideoTranscoder;

struct StreamMapping {
    mapping: Vec<isize>,
    video_transcoder: Option<VideoTranscoder>,
    audio_transcoder: Option<AudioTranscoder>,
}

fn setup_passthrough_stream(
    octx: &mut ffmpeg::format::context::Output,
    ist: &ffmpeg::format::stream::Stream,
) -> Result<(), String> {
    let mut ost = octx
        .add_stream(ffmpeg::encoder::find(ffmpeg::codec::Id::None))
        .map_err(|e| format!("add stream: {e}"))?;
    ost.set_parameters(ist.parameters());
    ost.set_time_base(ist.time_base());
    unsafe {
        (*ost.parameters().as_mut_ptr()).codec_tag = 0;
    }
    Ok(())
}

fn process_stream(
    octx: &mut ffmpeg::format::context::Output,
    ist_index: usize,
    ist: &ffmpeg::format::stream::Stream,
    ost_index: usize,
    config: &EncodeConfig,
    stream_mapping: &mut [isize],
) -> Result<(Option<VideoTranscoder>, Option<AudioTranscoder>), String> {
    let medium = ist.parameters().medium();
    let codec_id = ist.parameters().id();

    match medium {
        ffmpeg::media::Type::Video => {
            if codec_id == ffmpeg::codec::Id::AV1 {
                setup_passthrough_stream(octx, ist)?;
                stream_mapping[ist_index] = ost_index as isize;
                Ok((None, None))
            } else {
                let transcoder = VideoTranscoder::new(ist, octx, ost_index, config)
                    .map_err(|e| format!("init video transcoder: {e}"))?;
                stream_mapping[ist_index] = ost_index as isize;
                Ok((Some(transcoder), None))
            }
        }
        ffmpeg::media::Type::Audio => {
            if codec_id == ffmpeg::codec::Id::OPUS {
                setup_passthrough_stream(octx, ist)?;
                stream_mapping[ist_index] = ost_index as isize;
                Ok((None, None))
            } else {
                let transcoder = AudioTranscoder::new(ist, octx, ost_index)
                    .map_err(|e| format!("init audio transcoder: {e}"))?;
                stream_mapping[ist_index] = ost_index as isize;
                Ok((None, Some(transcoder)))
            }
        }
        _ => {
            stream_mapping[ist_index] = -1;
            Ok((None, None))
        }
    }
}

fn initialize_stream_mapping(
    ictx: &mut ffmpeg::format::context::Input,
    octx: &mut ffmpeg::format::context::Output,
    config: &EncodeConfig,
) -> Result<StreamMapping, String> {
    let nb_streams = ictx.nb_streams() as usize;
    let mut mapping = vec![-1; nb_streams];
    let mut ost_index = 0;
    let mut video_transcoder = None;
    let mut audio_transcoder = None;

    for (ist_index, ist) in ictx.streams().enumerate() {
        let (vt, at) = process_stream(octx, ist_index, &ist, ost_index, config, &mut mapping)?;

        if vt.is_some() || at.is_some() || matches!(ist.parameters().medium(), ffmpeg::media::Type::Video | ffmpeg::media::Type::Audio) {
            ost_index += 1;
        }

        if let Some(v) = vt {
            video_transcoder = Some(v);
        }
        if let Some(a) = at {
            audio_transcoder = Some(a);
        }
    }

    Ok(StreamMapping {
        mapping,
        video_transcoder,
        audio_transcoder,
    })
}

struct PacketContext<'a> {
    stream: &'a ffmpeg::format::stream::Stream<'a>,
    packet: &'a ffmpeg::Packet,
    stream_mapping: &'a [isize],
    video_transcoder: &'a mut Option<VideoTranscoder>,
    audio_transcoder: &'a mut Option<AudioTranscoder>,
    octx: &'a mut ffmpeg::format::context::Output,
    progress_cb: &'a dyn Fn(f64, String),
    total_duration: f64,
}

fn process_packet(ctx: &mut PacketContext<'_>) {
    let ist_index = ctx.stream.index();
    let ost_index_val = ctx.stream_mapping[ist_index];

    if ost_index_val < 0 {
        return;
    }

    let medium = ctx.stream.parameters().medium();
    let tb = f64::from(ctx.stream.time_base());
    let pts_val = ctx.packet.pts().or_else(|| ctx.packet.dts());

    match medium {
        ffmpeg::media::Type::Video if ctx.video_transcoder.is_some() => {
            if let Some(vt) = ctx.video_transcoder {
                vt.send_packet(ctx.packet);
                vt.receive_and_write(ctx.octx, ctx.progress_cb, ctx.total_duration);
            }
        }
        ffmpeg::media::Type::Audio if ctx.audio_transcoder.is_some() => {
            if let Some(at) = ctx.audio_transcoder {
                at.send_packet(ctx.packet);
                at.receive_and_write(ctx.octx);
            }
        }
        _ => {
            let mut pkt = ctx.packet.clone();
            let in_tb = ctx.stream.time_base();
            let out_tb = ctx
                .octx
                .stream(ost_index_val as usize)
                .map(|s| s.time_base())
                .unwrap_or(ffmpeg::Rational(1, 90000));
            pkt.rescale_ts(in_tb, out_tb);
            pkt.set_position(-1);
            pkt.set_stream(ost_index_val as usize);
            let _ = pkt.write_interleaved(ctx.octx);
        }
    }

    if ctx.total_duration > 0.0
        && let Some(ts) = pts_val
    {
        let current = ts as f64 * tb;
        let pct = (current / ctx.total_duration * 100.0).min(100.0);
        (ctx.progress_cb)(pct, super::format_duration(current));
    }
}

fn flush_transcoders(
    video_transcoder: &mut Option<VideoTranscoder>,
    audio_transcoder: &mut Option<AudioTranscoder>,
    octx: &mut ffmpeg::format::context::Output,
) {
    if let Some(vt) = video_transcoder {
        vt.flush(octx);
    }
    if let Some(at) = audio_transcoder {
        at.flush(octx);
    }
}

pub fn convert(
    input: PathBuf,
    output: PathBuf,
    config: EncodeConfig,
    info: &InputInfo,
    progress_cb: impl Fn(f64, String),
) -> Result<(), String> {
    ffmpeg::init().map_err(|e| format!("ffmpeg init: {e}"))?;

    let mut ictx = ffmpeg::format::input(&input).map_err(|e| format!("open input: {e}"))?;
    let mut octx = ffmpeg::format::output(&output).map_err(|e| format!("open output: {e}"))?;

    let StreamMapping {
        mapping,
        mut video_transcoder,
        mut audio_transcoder,
    } = initialize_stream_mapping(&mut ictx, &mut octx, &config)?;

    octx.set_metadata(ictx.metadata().to_owned());
    octx.write_header().map_err(|e| format!("write header: {e}"))?;

    let total_duration = info.duration_secs;

    for (stream, packet) in ictx.packets() {
        process_packet(&mut PacketContext {
            stream: &stream,
            packet: &packet,
            stream_mapping: &mapping,
            video_transcoder: &mut video_transcoder,
            audio_transcoder: &mut audio_transcoder,
            octx: &mut octx,
            progress_cb: &progress_cb,
            total_duration,
        });
    }

    flush_transcoders(&mut video_transcoder, &mut audio_transcoder, &mut octx);

    octx.write_trailer().map_err(|e| format!("write trailer: {e}"))?;

    progress_cb(100.0, "done".into());
    Ok(())
}
