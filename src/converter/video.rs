use ffmpeg_next as ffmpeg;

use super::{EncodeConfig, EncodeMode};

pub struct VideoTranscoder {
    decoder: ffmpeg::decoder::Video,
    encoder: ffmpeg::encoder::Video,
    input_time_base: ffmpeg::Rational,
    pub ost_index: usize,
}

impl VideoTranscoder {
    pub fn new(
        ist: &ffmpeg::format::stream::Stream,
        octx: &mut ffmpeg::format::context::Output,
        ost_index: usize,
        config: &EncodeConfig,
    ) -> Result<Self, ffmpeg::Error> {
        let context = ffmpeg::codec::context::Context::from_parameters(ist.parameters())?;
        let decoder = context.decoder().video()?;

        let codec =
            ffmpeg::encoder::find_by_name("libsvtav1").ok_or(ffmpeg::Error::InvalidData)?;

        let global = octx
            .format()
            .flags()
            .contains(ffmpeg::format::Flags::GLOBAL_HEADER);

        let mut ost = octx.add_stream(codec)?;

        let mut encoder = ffmpeg::codec::context::Context::new_with_codec(codec)
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

    pub fn send_packet(&mut self, packet: &ffmpeg::Packet) {
        let _ = self.decoder.send_packet(packet);
    }

    pub fn receive_and_write(
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
                    progress_cb(pct, super::format_duration(current));
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

    pub fn flush(&mut self, octx: &mut ffmpeg::format::context::Output) {
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
