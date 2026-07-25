use std::path::{Path, PathBuf};

use ffmpeg_next as ffmpeg;

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

pub fn detect_input(path: &Path) -> Result<InputInfo, String> {
    ffmpeg::init().map_err(|e| format!("ffmpeg init: {e}"))?;

    let ictx =
        ffmpeg::format::input(path).map_err(|e| format!("open input: {e}"))?;

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

struct VideoTranscoder {
    decoder: ffmpeg::decoder::Video,
    encoder: ffmpeg::encoder::Video,
    input_time_base: ffmpeg::Rational,
    ost_index: usize,
}

impl VideoTranscoder {
    fn new(
        ist: &ffmpeg::format::stream::Stream,
        octx: &mut ffmpeg::format::context::Output,
        ost_index: usize,
        config: &EncodeConfig,
    ) -> Result<Self, ffmpeg::Error> {
        let context =
            ffmpeg::codec::context::Context::from_parameters(ist.parameters())?;
        let decoder = context.decoder().video()?;

        let codec = ffmpeg::encoder::find_by_name("libsvtav1")
            .ok_or(ffmpeg::Error::InvalidData)?;

        let global = octx
            .format()
            .flags()
            .contains(ffmpeg::format::Flags::GLOBAL_HEADER);

        let mut ost = octx.add_stream(codec)?;

        let mut encoder =
            ffmpeg::codec::context::Context::new_with_codec(codec)
                .encoder()
                .video()?;

        encoder.set_width(decoder.width());
        encoder.set_height(decoder.height());
        encoder.set_format(decoder.format());
        encoder.set_frame_rate(decoder.frame_rate());
        encoder.set_time_base(ist.time_base());

        if global {
            encoder.set_flags(ffmpeg::codec::flag::Flags::GLOBAL_HEADER);
        }

        let mut opts = ffmpeg::Dictionary::new();
        opts.set("preset", "6");
        match config.mode {
            EncodeMode::CRF => {
                opts.set("crf", &config.crf_value.to_string());
            }
            EncodeMode::CBR => {
                opts.set("b:v", &format!("{}k", config.bitrate_kbps));
            }
        }

        let opened = encoder.open_with(opts)?;
        ost.set_parameters(&opened);

        Ok(Self {
            decoder,
            encoder: opened,
            input_time_base: ist.time_base(),
            ost_index,
        })
    }

    fn send_packet(&mut self, packet: &ffmpeg::Packet) {
        let _ = self.decoder.send_packet(packet);
    }

    fn receive_and_write(
        &mut self,
        octx: &mut ffmpeg::format::context::Output,
        progress_cb: &dyn Fn(f64, String),
        total_duration: f64,
    ) {
        let mut frame = ffmpeg::frame::Video::empty();

        while self.decoder.receive_frame(&mut frame).is_ok() {
            if total_duration > 0.0 {
                if let Some(pts) = frame.timestamp() {
                    let current = pts as f64 * f64::from(self.input_time_base);
                    let pct = (current / total_duration * 100.0).min(100.0);
                    progress_cb(pct, format_duration(current));
                }
            }
            let pts = frame.timestamp();
            frame.set_pts(pts);
            frame.set_kind(ffmpeg::picture::Type::None);
            let _ = self.encoder.send_frame(&frame);
            self.flush_encoder(octx);
        }
    }

    fn flush_encoder(&mut self, octx: &mut ffmpeg::format::context::Output) {
        let out_tb = octx
            .stream(self.ost_index)
            .map(|s| s.time_base())
            .unwrap_or(ffmpeg::Rational(1, 90000));
        let mut encoded = ffmpeg::Packet::empty();
        while self.encoder.receive_packet(&mut encoded).is_ok() {
            encoded.set_stream(self.ost_index);
            encoded.rescale_ts(self.input_time_base, out_tb);
            let _ = encoded.write_interleaved(octx);
        }
    }

    fn flush(&mut self, octx: &mut ffmpeg::format::context::Output) {
        let _ = self.decoder.send_eof();
        let mut frame = ffmpeg::frame::Video::empty();
        while self.decoder.receive_frame(&mut frame).is_ok() {
            let pts = frame.timestamp();
            frame.set_pts(pts);
            frame.set_kind(ffmpeg::picture::Type::None);
            let _ = self.encoder.send_frame(&frame);
            self.flush_encoder(octx);
        }
        let _ = self.encoder.send_eof();
        self.flush_encoder(octx);
    }
}

struct AudioTranscoder {
    decoder: ffmpeg::decoder::Audio,
    encoder: ffmpeg::encoder::Audio,
    input_time_base: ffmpeg::Rational,
    ost_index: usize,
}

impl AudioTranscoder {
    fn new(
        ist: &ffmpeg::format::stream::Stream,
        octx: &mut ffmpeg::format::context::Output,
        ost_index: usize,
    ) -> Result<Self, ffmpeg::Error> {
        let context =
            ffmpeg::codec::context::Context::from_parameters(ist.parameters())?;
        let decoder = context.decoder().audio()?;

        let codec = ffmpeg::encoder::find(ffmpeg::codec::Id::OPUS)
            .ok_or(ffmpeg::Error::InvalidData)?;

        let global = octx
            .format()
            .flags()
            .contains(ffmpeg::format::Flags::GLOBAL_HEADER);

        let mut ost = octx.add_stream(codec)?;

        let mut encoder =
            ffmpeg::codec::context::Context::new_with_codec(codec)
                .encoder()
                .audio()?;

        let channel_layout = decoder.channel_layout();

        encoder.set_rate(decoder.rate() as i32);
        encoder.set_channel_layout(channel_layout);
        encoder.set_time_base((1, decoder.rate() as i32));
        encoder.set_bit_rate(128000);

        if global {
            encoder.set_flags(ffmpeg::codec::flag::Flags::GLOBAL_HEADER);
        }

        let opened = encoder.open_as(codec)?;
        let tb = (1, decoder.rate() as i32);
        ost.set_parameters(&opened);
        ost.set_time_base(tb);

        Ok(Self {
            decoder,
            encoder: opened,
            input_time_base: ist.time_base(),
            ost_index,
        })
    }

    fn send_packet(&mut self, packet: &ffmpeg::Packet) {
        let _ = self.decoder.send_packet(packet);
    }

    fn receive_and_write(
        &mut self,
        octx: &mut ffmpeg::format::context::Output,
    ) {
        let mut frame = ffmpeg::frame::Audio::empty();

        while self.decoder.receive_frame(&mut frame).is_ok() {
            let pts = frame.timestamp();
            frame.set_pts(pts);
            let _ = self.encoder.send_frame(&frame);
            self.flush_encoder(octx);
        }
    }

    fn flush_encoder(&mut self, octx: &mut ffmpeg::format::context::Output) {
        let out_tb = octx
            .stream(self.ost_index)
            .map(|s| s.time_base())
            .unwrap_or(ffmpeg::Rational(1, 90000));
        let mut encoded = ffmpeg::Packet::empty();
        while self.encoder.receive_packet(&mut encoded).is_ok() {
            encoded.set_stream(self.ost_index);
            encoded.rescale_ts(self.input_time_base, out_tb);
            let _ = encoded.write_interleaved(octx);
        }
    }

    fn flush(&mut self, octx: &mut ffmpeg::format::context::Output) {
        let _ = self.decoder.send_eof();
        let mut frame = ffmpeg::frame::Audio::empty();
        while self.decoder.receive_frame(&mut frame).is_ok() {
            let pts = frame.timestamp();
            frame.set_pts(pts);
            let _ = self.encoder.send_frame(&frame);
            self.flush_encoder(octx);
        }
        let _ = self.encoder.send_eof();
        self.flush_encoder(octx);
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

    let mut ictx =
        ffmpeg::format::input(&input).map_err(|e| format!("open input: {e}"))?;

    let mut octx = ffmpeg::format::output(&output)
        .map_err(|e| format!("open output: {e}"))?;

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
                        .add_stream(ffmpeg::encoder::find(
                            ffmpeg::codec::Id::None,
                        ))
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
                            .map_err(|e| {
                                format!("init video transcoder: {e}")
                            })?;
                    stream_mapping[ist_index] = ost_index as isize;
                    video_transcoder = Some(transcoder);
                }
                ost_index += 1;
            }
            ffmpeg::media::Type::Audio => {
                if codec_id == ffmpeg::codec::Id::OPUS {
                    let mut ost = octx
                        .add_stream(ffmpeg::encoder::find(
                            ffmpeg::codec::Id::None,
                        ))
                        .map_err(|e| format!("add stream: {e}"))?;
                    ost.set_parameters(ist.parameters());
                    ost.set_time_base(ist.time_base());
                    unsafe {
                        (*ost.parameters().as_mut_ptr()).codec_tag = 0;
                    }
                    stream_mapping[ist_index] = ost_index as isize;
                } else {
                    let transcoder =
                        AudioTranscoder::new(&ist, &mut octx, ost_index)
                            .map_err(|e| {
                                format!("init audio transcoder: {e}")
                            })?;
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
                progress_cb(pct, format_duration(current));
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

fn format_duration(secs: f64) -> String {
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
