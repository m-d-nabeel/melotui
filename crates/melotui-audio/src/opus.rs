//! Opus, which the decoders the rest go through have nothing for: read out
//! of its Ogg file by Symphonia, and decoded by `rusty-opus`.

use std::{fs::File, time::Duration};

use rodio::{Decoder, Source, source::SeekError};
use rusty_opus::OpusDecoder;
use symphonia::core::{
    codecs::CODEC_TYPE_OPUS,
    formats::{FormatOptions, FormatReader, SeekMode, SeekTo},
    io::MediaSourceStream,
    units::Time,
};
use symphonia::default::formats::OggReader;

/// Opus is decoded at this rate whatever it was recorded at.
const RATE: u32 = 48_000;

/// The frames of the longest packet there can be: 120 ms.
const LONGEST_PACKET: usize = 5_760;

/// What a decoder takes to forget what came before where it was moved to:
/// 80 ms, played into it unheard.
const RUN_UP: u64 = 3_840;

/// The audio of an Ogg Opus file, mono or stereo.
pub struct OpusSource {
    reader: OggReader,
    decoder: OpusDecoder,
    track: u32,
    channels: usize,
    /// The frames there are, once what the encoder padded with is taken off.
    frames: Option<u64>,
    /// The packet decoded last, and how far through it playback is.
    samples: Vec<f32>,
    at: usize,
    /// The frames the encoder put before the audio, for its decoder to
    /// run up on: decoded and thrown away, and counted in every time the
    /// file gives.
    lead_in: u64,
    /// Frames still to be decoded and thrown away, at the start and after
    /// a move.
    discard: u64,
    scratch: Vec<f32>,
}

impl OpusSource {
    /// Opens `file` if it is Ogg Opus in one or two channels; says why not
    /// otherwise.
    pub fn open(file: File) -> Result<Self, String> {
        let stream = MediaSourceStream::new(Box::new(file), Default::default());
        // With what the encoder padded the end with marked on the last
        // packet, to be left out.
        let options = FormatOptions {
            enable_gapless: true,
            ..Default::default()
        };
        let reader = OggReader::try_new(stream, &options).map_err(|e| e.to_string())?;
        let track = reader
            .tracks()
            .iter()
            .find(|track| track.codec_params.codec == CODEC_TYPE_OPUS)
            .ok_or("there is no Opus in it")?;
        let params = &track.codec_params;
        let channels = params.channels.map_or(0, |channels| channels.count());
        if !(1..=2).contains(&channels) {
            return Err(format!("Opus in {channels} channels is not played"));
        }
        let decoder = OpusDecoder::new(RATE as i32, channels).map_err(|e| e.to_string())?;
        let lead_in = u64::from(params.delay.unwrap_or(0));
        Ok(Self {
            track: track.id,
            frames: params.n_frames.map(|frames| frames.saturating_sub(lead_in)),
            lead_in,
            discard: lead_in,
            reader,
            decoder,
            channels,
            samples: Vec::new(),
            at: 0,
            scratch: vec![0.0; LONGEST_PACKET * channels],
        })
    }

    /// Decodes the next packet with anything in it to play. `false` at the
    /// end of the file.
    fn refill(&mut self) -> bool {
        loop {
            let Ok(packet) = self.reader.next_packet() else {
                return false;
            };
            if packet.track_id() != self.track {
                continue;
            }
            // A packet that cannot be decoded is passed over: a moment's
            // gap, where stopping would end the song.
            // Nor is one the decoder gives up on altogether: this is the
            // thread the music is played from, and a panic here would be
            // the end of all playback.
            let decoded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                self.decoder
                    .decode(&packet.data, LONGEST_PACKET, &mut self.scratch)
            }));
            let frames = match decoded {
                Ok(Ok(frames)) => frames,
                Ok(Err(_)) => continue,
                Err(_) => {
                    // Whatever state it was left in is not to be decoded on from.
                    if let Ok(fresh) = OpusDecoder::new(RATE as i32, self.channels) {
                        self.decoder = fresh;
                    }
                    continue;
                }
            };
            // Left out: the encoder's padding, and after a move the run-up
            // to where it was moved to.
            let skip = (u64::from(packet.trim_start) + self.discard).min(frames as u64);
            self.discard -= skip.min(self.discard);
            let keep = frames
                .saturating_sub(packet.trim_end as usize)
                .max(skip as usize);
            let (from, to) = (skip as usize * self.channels, keep * self.channels);
            if from < to {
                self.samples.clear();
                self.samples.extend_from_slice(&self.scratch[from..to]);
                self.at = 0;
                return true;
            }
        }
    }
}

impl Iterator for OpusSource {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        if self.at >= self.samples.len() && !self.refill() {
            return None;
        }
        let sample = self.samples[self.at];
        self.at += 1;
        Some(sample)
    }
}

impl Source for OpusSource {
    fn current_span_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> rodio::ChannelCount {
        rodio::ChannelCount::new(self.channels as u16).unwrap_or(rodio::nz!(2))
    }
    fn sample_rate(&self) -> rodio::SampleRate {
        rodio::nz!(48_000)
    }
    fn total_duration(&self) -> Option<Duration> {
        self.frames
            .map(|frames| Duration::from_secs_f64(frames as f64 / f64::from(RATE)))
    }
    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        // Asked for a little before where it is going, for the run-up.
        let wanted = (pos.as_secs_f64() * f64::from(RATE)) as u64 + self.lead_in;
        let from = wanted.saturating_sub(RUN_UP);
        let time = Time::from(from as f64 / f64::from(RATE));
        let to = SeekTo::Time {
            time,
            track_id: Some(self.track),
        };
        let moved = self
            .reader
            .seek(SeekMode::Accurate, to)
            .map_err(|e| SeekError::Other(std::sync::Arc::new(e)))?;
        if let Ok(fresh) = OpusDecoder::new(RATE as i32, self.channels) {
            self.decoder = fresh;
        }
        self.samples.clear();
        self.at = 0;
        // The packet it landed on starts at or before what was asked for.
        self.discard = wanted.saturating_sub(moved.actual_ts);
        Ok(())
    }
}

/// A file being decoded: by the decoders most formats go through, or as
/// Opus.
pub enum Decoded {
    Common(Decoder<std::io::BufReader<File>>),
    Opus(Box<OpusSource>),
}

impl Decoded {
    /// Opens the file at `path` with whichever decodes it. The error is
    /// the common decoders', unless the file is Opus that cannot be played.
    pub fn open(path: &std::path::Path) -> Result<Self, String> {
        let common = match Decoder::try_from(File::open(path).map_err(|e| e.to_string())?) {
            Ok(decoder) => return Ok(Self::Common(decoder)),
            Err(e) => e.to_string(),
        };
        let file = File::open(path).map_err(|e| e.to_string())?;
        match OpusSource::open(file) {
            Ok(opus) => Ok(Self::Opus(Box::new(opus))),
            Err(why) if why.starts_with("Opus in") => Err(why),
            Err(_) => Err(common),
        }
    }
}

impl Iterator for Decoded {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<f32> {
        match self {
            Self::Common(decoder) => decoder.next(),
            Self::Opus(opus) => opus.next(),
        }
    }
}

impl Source for Decoded {
    fn current_span_len(&self) -> Option<usize> {
        match self {
            Self::Common(decoder) => decoder.current_span_len(),
            Self::Opus(opus) => opus.current_span_len(),
        }
    }
    fn channels(&self) -> rodio::ChannelCount {
        match self {
            Self::Common(decoder) => decoder.channels(),
            Self::Opus(opus) => opus.channels(),
        }
    }
    fn sample_rate(&self) -> rodio::SampleRate {
        match self {
            Self::Common(decoder) => decoder.sample_rate(),
            Self::Opus(opus) => opus.sample_rate(),
        }
    }
    fn total_duration(&self) -> Option<Duration> {
        match self {
            Self::Common(decoder) => decoder.total_duration(),
            Self::Opus(opus) => opus.total_duration(),
        }
    }
    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        match self {
            Self::Common(decoder) => decoder.try_seek(pos),
            Self::Opus(opus) => opus.try_seek(pos),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    fn open(name: &str) -> Decoded {
        Decoded::open(&fixture(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
    }

    #[test]
    fn opus_is_played_all_of_it_and_nothing_the_encoder_padded_it_with() {
        // Three seconds of two tones, one a side.
        let source = open("tone.opus");
        assert!(matches!(source, Decoded::Opus(_)));
        assert_eq!(source.channels().get(), 2);
        assert_eq!(source.sample_rate().get(), 48_000);
        assert_eq!(source.total_duration(), Some(Duration::from_secs(3)));

        let samples: Vec<f32> = source.collect();
        assert_eq!(samples.len(), 3 * 48_000 * 2);
        // The tones, and not silence or noise: sines a sixteenth of full
        // scale high, whose root mean square is 0.044.
        let loudness = |side: usize| {
            let of_side = samples.iter().skip(side).step_by(2);
            (of_side.map(|s| s * s).sum::<f32>() / (samples.len() / 2) as f32).sqrt()
        };
        for side in [0, 1] {
            assert!((0.04..0.05).contains(&loudness(side)), "{}", loudness(side));
        }
        // 440 Hz on the left and 880 Hz on the right: twice the crossings.
        let crossings = |side: usize| {
            let of_side: Vec<f32> = samples.iter().skip(side).step_by(2).copied().collect();
            of_side
                .windows(2)
                .filter(|w| w[0] < 0.0 && w[1] >= 0.0)
                .count()
        };
        assert!((1_310..=1_330).contains(&crossings(0)), "{}", crossings(0));
        assert!((2_630..=2_650).contains(&crossings(1)), "{}", crossings(1));
    }

    #[test]
    fn opus_in_one_channel_is_played() {
        let source = open("tone-mono.opus");
        assert_eq!(source.channels().get(), 1);
        assert_eq!(source.count(), 48_000);
    }

    #[test]
    fn opus_is_moved_through_to_the_frame_asked_for() {
        let whole: Vec<f32> = open("tone.opus").collect();
        for secs in [2.0, 0.5, 0.0, 2.9] {
            let mut source = open("tone.opus");
            // From part way in, as it is when playing.
            for _ in 0..10_000 {
                source.next();
            }
            source.try_seek(Duration::from_secs_f64(secs)).unwrap();
            let rest: Vec<f32> = source.collect();
            let from = (secs * 48_000.0) as usize * 2;
            assert_eq!(rest.len(), whole.len() - from, "moved to {secs}");
            // The same audio as playing through to there, the decoder
            // having run up to it.
            let apart = rest
                .iter()
                .zip(&whole[from..])
                .map(|(a, b)| (a - b).abs())
                .fold(0.0, f32::max);
            assert!(apart < 0.02, "moved to {secs}: {apart} apart");
        }
        // Past the end, there is nothing left to play.
        let mut source = open("tone.opus");
        if source.try_seek(Duration::from_secs(60)).is_ok() {
            assert!(source.count() < 48_000);
        }
    }

    #[test]
    fn opus_in_more_channels_than_two_says_so() {
        let Err(why) = Decoded::open(&fixture("tone-surround.opus")) else {
            panic!("it was opened");
        };
        assert_eq!(why, "Opus in 6 channels is not played");
    }

    #[test]
    fn what_is_not_opus_goes_through_the_common_decoders_and_fails_as_they_say() {
        assert!(matches!(open("tone.m4a"), Decoded::Common(_)));

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.opus");
        std::fs::write(&path, "not audio at all").unwrap();
        let Err(why) = Decoded::open(&path) else {
            panic!("it was opened");
        };
        assert!(!why.contains("Opus"), "{why}");
    }

    /// Run by hand: `OPUS=file.opus OUT=file.f32 cargo test ... -- --ignored`
    /// writes what is decoded, to be compared with another decoder's.
    #[test]
    #[ignore = "writes a file for comparing by hand"]
    fn dump_what_is_decoded() {
        let source = Decoded::open(std::env::var("OPUS").unwrap().as_ref()).unwrap();
        let bytes: Vec<u8> = source.flat_map(f32::to_le_bytes).collect();
        std::fs::write(std::env::var("OUT").unwrap(), bytes).unwrap();
    }
}
