use ffmpeg_next as ffmpeg;

pub struct AudioTranscoder {
    decoder: ffmpeg::decoder::Audio,
    encoder: ffmpeg::encoder::Audio,
    input_time_base: ffmpeg::Rational,
    pub ost_index: usize,
}

impl AudioTranscoder {
    pub fn new(
        ist: &ffmpeg::format::stream::Stream,
        octx: &mut ffmpeg::format::context::Output,
        ost_index: usize,
    ) -> Result<Self, ffmpeg::Error> {
        let context = ffmpeg::codec::context::Context::from_parameters(ist.parameters())?;
        let decoder = context.decoder().audio()?;

        let codec = ffmpeg::encoder::find(ffmpeg::codec::Id::OPUS)
            .ok_or(ffmpeg::Error::InvalidData)?;

        let global = octx
            .format()
            .flags()
            .contains(ffmpeg::format::Flags::GLOBAL_HEADER);

        let mut ost = octx.add_stream(codec)?;

        let mut encoder = ffmpeg::codec::context::Context::new_with_codec(codec)
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

    pub fn send_packet(&mut self, packet: &ffmpeg::Packet) {
        let _ = self.decoder.send_packet(packet);
    }

    pub fn receive_and_write(&mut self, octx: &mut ffmpeg::format::context::Output) -> Result<(), String> {
        let mut frame = ffmpeg::frame::Audio::empty();

        while self.decoder.receive_frame(&mut frame).is_ok() {
            let pts = frame.timestamp();
            frame.set_pts(pts);
            self.encoder
                .send_frame(&frame)
                .map_err(|e| format!("audio send_frame: {e}"))?;
            self.flush_encoder(octx)?;
        }
        Ok(())
    }

    fn flush_encoder(&mut self, octx: &mut ffmpeg::format::context::Output) -> Result<(), String> {
        let out_tb = octx
            .stream(self.ost_index)
            .map(|s| s.time_base())
            .unwrap_or(ffmpeg::Rational(1, 90000));
        let mut encoded = ffmpeg::Packet::empty();
        while self.encoder.receive_packet(&mut encoded).is_ok() {
            encoded.set_stream(self.ost_index);
            encoded.rescale_ts(self.input_time_base, out_tb);
            encoded
                .write_interleaved(octx)
                .map_err(|e| format!("audio write: {e}"))?;
        }
        Ok(())
    }

    pub fn flush(&mut self, octx: &mut ffmpeg::format::context::Output) -> Result<(), String> {
        self.decoder
            .send_eof()
            .map_err(|e| format!("audio decoder flush: {e}"))?;
        let mut frame = ffmpeg::frame::Audio::empty();
        while self.decoder.receive_frame(&mut frame).is_ok() {
            let pts = frame.timestamp();
            frame.set_pts(pts);
            self.encoder
                .send_frame(&frame)
                .map_err(|e| format!("audio flush send_frame: {e}"))?;
            self.flush_encoder(octx)?;
        }
        self.encoder
            .send_eof()
            .map_err(|e| format!("audio encoder flush: {e}"))?;
        self.flush_encoder(octx)?;
        Ok(())
    }
}
