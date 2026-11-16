//! Opus, which the decoders the rest go through have nothing for: read out
//! of its Ogg file by Symphonia, and decoded by `opus-rs`. And where
//! each file's channels are to be heard from, for mixing them down.

use std::{fs::File, time::Duration};

use opus_rs::{
    OpusDecoder,
    multistream::{ChannelMappingTable, MultistreamDecoder},
};
use rodio::{Decoder, Source, source::SeekError};
use symphonia::core::{
    audio::Channels,
    codecs::CODEC_TYPE_OPUS,
    formats::{FormatOptions, FormatReader, SeekMode, SeekTo},
    io::MediaSourceStream,
    meta::MetadataOptions,
    probe::Hint,
    units::Time,
};
use symphonia::default::formats::OggReader;

/// Opus is decoded at this rate whatever it was recorded at.
const RATE: u32 = 48_000;

/// A channel of surround that is silent rather than taken from a stream
/// (RFC 7845, section 5.1.1).
const SILENT_CHANNEL: u8 = 255;

/// The frames of the longest packet there can be: 120 ms.
const LONGEST_PACKET: usize = 5_760;

/// What a decoder takes to forget what came before where it was moved to:
/// 80 ms, played into it unheard.
const RUN_UP: u64 = 3_840;

/// The decoder for one stream of one or two channels, or for the streams
/// surround is coded in.
enum Codec {
    Plain(Box<OpusDecoder>),
    /// The streams, shared out among the channels as the file's header says.
    Surround(Box<MultistreamDecoder>),
}

impl Codec {
    fn new(channels: usize, table: Option<&ChannelMappingTable>) -> Result<Self, String> {
        match table {
            Some(table) => MultistreamDecoder::new(RATE as i32, table.clone())
                .map(|decoder| Self::Surround(Box::new(decoder))),
            None => OpusDecoder::new(RATE as i32, channels)
                .map(|decoder| Self::Plain(Box::new(decoder))),
        }
        .map_err(str::to_string)
    }

    fn decode(&mut self, packet: &[u8], out: &mut [f32]) -> Result<usize, &'static str> {
        match self {
            Self::Plain(decoder) => decoder.decode(packet, LONGEST_PACKET, out),
            Self::Surround(decoder) => decoder.decode(packet, LONGEST_PACKET, out),
        }
    }
}

/// The speakers of Opus's surround (channel mapping family 1) in the order
/// its channels come, which is Vorbis's (RFC 7845, section 5.1.1.2).
fn vorbis_speakers(count: usize) -> Option<Vec<Channels>> {
    use Channels as C;
    let (fl, fr, fc, lfe) = (C::FRONT_LEFT, C::FRONT_RIGHT, C::FRONT_CENTRE, C::LFE1);
    let (rl, rr, rc, sl, sr) = (
        C::REAR_LEFT,
        C::REAR_RIGHT,
        C::REAR_CENTRE,
        C::SIDE_LEFT,
        C::SIDE_RIGHT,
    );
    Some(match count {
        3 => vec![fl, fc, fr],
        4 => vec![fl, fr, rl, rr],
        5 => vec![fl, fc, fr, rl, rr],
        6 => vec![fl, fc, fr, rl, rr, lfe],
        7 => vec![fl, fc, fr, sl, sr, rc, lfe],
        8 => vec![fl, fc, fr, sl, sr, rl, rr, lfe],
        _ => return None,
    })
}

/// What the identification header (RFC 7845, section 5.1) of a file with
/// `channels` says that Symphonia does not pass on: the gain everything is
/// to be played at, as a factor, and for surround (channel mapping family 1)
/// how its channels are shared out among its streams — whatever way that
/// is, as the RFC says a player must take any.
fn read_head(head: &[u8], channels: usize) -> Result<(f32, Option<ChannelMappingTable>), String> {
    if head.len() < 19 || &head[..8] != b"OpusHead" {
        return Err("its header is not Opus's".into());
    }
    // Q7.8 decibels, which the RFC says a player MUST apply.
    let decibels = f32::from(i16::from_le_bytes([head[16], head[17]])) / 256.0;
    let gain = 10f32.powf(decibels / 20.0);
    let family = head[18];
    if family == 0 {
        return Ok((gain, None));
    }
    let invalid =
        || format!("Opus in {channels} channels with a channel mapping that is not valid");
    let table =
        ChannelMappingTable::parse(family, channels as u8, &head[19..]).ok_or_else(invalid)?;
    // Each channel taken from a channel one of the streams has, or silent;
    // the decoder indexes its streams' channels with these unchecked.
    let streams = usize::from(table.stream_count);
    let coupled = usize::from(table.coupled_count);
    let decoded = streams + coupled;
    let valid = streams > 0
        && coupled <= streams
        && table
            .mapping
            .iter()
            .all(|&from| from == SILENT_CHANNEL || usize::from(from) < decoded);
    if valid {
        Ok((gain, Some(table)))
    } else {
        Err(invalid())
    }
}

/// The audio of an Ogg Opus file, in up to eight channels.
pub struct OpusSource {
    reader: OggReader,
    decoder: Codec,
    /// How surround's channels are shared out among its streams, for a
    /// fresh decoder after a move.
    table: Option<ChannelMappingTable>,
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
    /// What every sample is multiplied by: the header's gain.
    gain: f32,
}

impl OpusSource {
    /// Opens `file` if it is Ogg Opus the decoder can play; says why not
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
        if !(1..=8).contains(&channels) {
            return Err(format!("Opus in {channels} channels is not played"));
        }
        let (gain, table) = read_head(params.extra_data.as_deref().unwrap_or_default(), channels)?;
        let decoder = Codec::new(channels, table.as_ref())?;
        let lead_in = u64::from(params.delay.unwrap_or(0));
        Ok(Self {
            track: track.id,
            frames: params.n_frames.map(|frames| frames.saturating_sub(lead_in)),
            lead_in,
            discard: lead_in,
            reader,
            decoder,
            table,
            channels,
            samples: Vec::new(),
            at: 0,
            scratch: vec![0.0; LONGEST_PACKET * channels],
            gain,
        })
    }

    /// Where each channel is to be heard from, if there are more than two.
    pub fn speakers(&self) -> Option<Vec<Channels>> {
        vorbis_speakers(self.channels)
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
                self.decoder.decode(&packet.data, &mut self.scratch)
            }));
            let frames = match decoded {
                Ok(Ok(frames)) => frames,
                Ok(Err(_)) => continue,
                Err(_) => {
                    // Whatever state it was left in is not to be decoded on from.
                    if let Ok(fresh) = Codec::new(self.channels, self.table.as_ref()) {
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
                let gain = self.gain;
                self.samples
                    .extend(self.scratch[from..to].iter().map(|sample| sample * gain));
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
        if let Ok(fresh) = Codec::new(self.channels, self.table.as_ref()) {
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
    Common {
        decoder: Decoder<std::io::BufReader<File>>,
        /// Where its channels are to be heard from, if it has more than two
        /// and the file says.
        speakers: Option<Vec<Channels>>,
    },
    Opus(Box<OpusSource>),
}

impl Decoded {
    /// Opens the file at `path` with whichever decodes it. The error is
    /// the common decoders', unless the file is Opus that cannot be played.
    pub fn open(path: &std::path::Path) -> Result<Self, String> {
        let common = match Decoder::try_from(File::open(path).map_err(|e| e.to_string())?) {
            Ok(decoder) => {
                let speakers = (decoder.channels().get() > 2)
                    .then(|| speakers_in(path))
                    .flatten();
                return Ok(Self::Common { decoder, speakers });
            }
            Err(e) => e.to_string(),
        };
        let file = File::open(path).map_err(|e| e.to_string())?;
        match OpusSource::open(file) {
            Ok(opus) => Ok(Self::Opus(Box::new(opus))),
            Err(why) if why.starts_with("Opus in") => Err(why),
            Err(_) => Err(common),
        }
    }

    /// Where each channel is to be heard from, where there are more than two
    /// and that is known.
    pub fn speakers(&self) -> Option<Vec<Channels>> {
        match self {
            Self::Common { speakers, .. } => speakers.clone(),
            Self::Opus(opus) => opus.speakers(),
        }
    }
}

/// The speakers the file at `path` says its channels are for, in the order
/// they are decoded in: the order of the flags, whatever order the file
/// keeps them in — the decoders put Vorbis's and AAC's in that order too.
/// `None` if it does not say, or cannot be read for it.
fn speakers_in(path: &std::path::Path) -> Option<Vec<Channels>> {
    let file = File::open(path).ok()?;
    let stream = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(extension) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(extension);
    }
    let mut probed = symphonia::default::get_probe()
        .format(
            &hint,
            stream,
            &FormatOptions::default(),
            &MetadataOptions::default(),
        )
        .ok()?;
    let channels = probed.format.default_track()?.codec_params.channels?;
    // FLAC has one fixed layout for each count of channels, and says it is
    // another with a tag (RFC 9639, section 8.6.2) — which only counts if
    // it has as many channels as there are.
    let tagged = channel_mask_tag(&mut probed).filter(|mask| mask.count() == channels.count());
    Some(crate::downmix::speakers_of(tagged.unwrap_or(channels)))
}

/// The speakers a `WAVEFORMATEXTENSIBLE_CHANNEL_MASK` tag says a file's
/// channels are for: WAVE's flags, the first eighteen of which are
/// Symphonia's too, written in hexadecimal after `0x`.
fn channel_mask_tag(probed: &mut symphonia::core::probe::ProbeResult) -> Option<Channels> {
    let tagged = |revision: &symphonia::core::meta::MetadataRevision| {
        revision.tags().iter().find_map(|tag| {
            tag.key
                .eq_ignore_ascii_case("WAVEFORMATEXTENSIBLE_CHANNEL_MASK")
                .then(|| tag.value.to_string())
        })
    };
    let value = probed
        .format
        .metadata()
        .current()
        .and_then(tagged)
        .or_else(|| probed.metadata.get()?.current().and_then(tagged))?;
    let value = value.trim();
    let bits = match value
        .strip_prefix("0x")
        .or_else(|| value.strip_prefix("0X"))
    {
        Some(hex) => u32::from_str_radix(hex, 16).ok()?,
        None => value.parse().ok()?,
    };
    // The eighteen speakers WAVE names; anything else is not a mask.
    if bits == 0 || bits & !0x3_FFFF != 0 {
        return None;
    }
    Channels::from_bits(bits)
}

impl Iterator for Decoded {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<f32> {
        match self {
            Self::Common { decoder, .. } => decoder.next(),
            Self::Opus(opus) => opus.next(),
        }
    }
}

impl Source for Decoded {
    fn current_span_len(&self) -> Option<usize> {
        match self {
            Self::Common { decoder, .. } => decoder.current_span_len(),
            Self::Opus(opus) => opus.current_span_len(),
        }
    }
    fn channels(&self) -> rodio::ChannelCount {
        match self {
            Self::Common { decoder, .. } => decoder.channels(),
            Self::Opus(opus) => opus.channels(),
        }
    }
    fn sample_rate(&self) -> rodio::SampleRate {
        match self {
            Self::Common { decoder, .. } => decoder.sample_rate(),
            Self::Opus(opus) => opus.sample_rate(),
        }
    }
    fn total_duration(&self) -> Option<Duration> {
        match self {
            Self::Common { decoder, .. } => decoder.total_duration(),
            Self::Opus(opus) => opus.total_duration(),
        }
    }
    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        match self {
            Self::Common { decoder, .. } => decoder.try_seek(pos),
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

    /// The loudness, as amplitude, of `frequency` in one channel of
    /// interleaved `samples`, over the middle of them.
    fn level(samples: &[f32], channels: usize, channel: usize, frequency: f64) -> f64 {
        let frames = samples.len() / channels;
        let (from, to) = (frames / 4, frames * 3 / 4);
        let w = std::f64::consts::TAU * frequency / 48_000.0;
        let (mut s1, mut s2) = (0f64, 0f64);
        for frame in from..to {
            let s0 = f64::from(samples[frame * channels + channel]) + 2.0 * w.cos() * s1 - s2;
            s2 = s1;
            s1 = s0;
        }
        2.0 * (s1 * s1 + s2 * s2 - 2.0 * w.cos() * s1 * s2)
            .max(0.0)
            .sqrt()
            / (to - from) as f64
    }

    /// The tone in each channel, of those the surround fixtures have: one
    /// for each speaker, the LFE's at 60 Hz.
    fn tones(samples: &[f32], channels: usize) -> Vec<u32> {
        (0..channels)
            .map(|channel| {
                [60, 300, 450, 600, 750, 900, 1050, 1200, 1350]
                    .into_iter()
                    .max_by(|&a, &b| {
                        let at = |f: u32| level(samples, channels, channel, f64::from(f));
                        at(a).total_cmp(&at(b))
                    })
                    .unwrap()
            })
            .collect()
    }

    #[test]
    fn surround_opus_comes_in_its_order_each_channel_from_its_speaker() {
        // FL 300, FR 450, FC 600, LFE 60, BL 900, BR 1050, SL 1200, SR 1350,
        // coded as libopus codes 7.1: in Vorbis's order, in five streams.
        let source = open("surround-7.1.opus");
        assert_eq!(source.channels().get(), 8);
        let speakers = source.speakers().unwrap();
        let samples: Vec<f32> = source.collect();
        assert_eq!(
            tones(&samples, 8),
            [300, 600, 450, 1200, 1350, 900, 1050, 60]
        );
        use Channels as C;
        assert_eq!(
            speakers,
            [
                C::FRONT_LEFT,
                C::FRONT_CENTRE,
                C::FRONT_RIGHT,
                C::SIDE_LEFT,
                C::SIDE_RIGHT,
                C::REAR_LEFT,
                C::REAR_RIGHT,
                C::LFE1
            ]
        );
        // Each at the level it was made at, not something else's noise.
        for channel in 0..7 {
            let tone = f64::from(tones(&samples, 8)[channel]);
            let at = level(&samples, 8, channel, tone);
            assert!((at - 0.125).abs() < 0.005, "channel {channel}: {at}");
        }
    }

    /// `file` with the bytes of its Opus header from `at` changed to `to`,
    /// and its first page's checksum made right again.
    fn with_header_changed(file: &str, at: usize, to: &[u8]) -> tempfile::NamedTempFile {
        let mut bytes = std::fs::read(fixture(file)).unwrap();
        assert_eq!(&bytes[..4], b"OggS");
        let segments = usize::from(bytes[26]);
        let head = 27 + segments;
        let page_len = head
            + bytes[27..head]
                .iter()
                .map(|&n| usize::from(n))
                .sum::<usize>();
        assert_eq!(&bytes[head..head + 8], b"OpusHead");
        bytes[head + at..head + at + to.len()].copy_from_slice(to);
        // Ogg's CRC-32: polynomial 0x04c11db7, from zero, not reflected,
        // over the page with its checksum taken as zero.
        bytes[22..26].fill(0);
        let mut crc = 0u32;
        for &byte in &bytes[..page_len] {
            crc ^= u32::from(byte) << 24;
            for _ in 0..8 {
                crc = if crc & 0x8000_0000 != 0 {
                    (crc << 1) ^ 0x04c1_1db7
                } else {
                    crc << 1
                };
            }
        }
        bytes[22..26].copy_from_slice(&crc.to_le_bytes());
        let changed = tempfile::Builder::new().suffix(".opus").tempfile().unwrap();
        std::fs::write(changed.path(), bytes).unwrap();
        changed
    }

    #[test]
    fn the_gain_the_header_gives_is_played_at() {
        // RFC 7845: a player MUST apply it. 1541/256 dB is twice as loud.
        let loud = with_header_changed("tone.opus", 16, &1541i16.to_le_bytes());
        let as_made: Vec<f32> = open("tone.opus").collect();
        let louder: Vec<f32> = Decoded::open(loud.path()).unwrap().collect();
        let ratio = level(&louder, 2, 0, 440.0) / level(&as_made, 2, 0, 440.0);
        assert!((ratio - 2.0).abs() < 0.01, "{ratio}");
    }

    #[test]
    fn surround_coded_another_way_is_decoded_as_its_header_says() {
        // libopus's 5.1 takes FL, FR from the first stream; this file says
        // the other way round. The RFC: a player MUST take any valid one.
        let swapped = with_header_changed("surround-5.1.opus", 21, &[1, 4, 0]);
        let source = Decoded::open(swapped.path()).unwrap();
        let samples: Vec<f32> = source.collect();
        // FL 300 and FR 450 where each other's channel is.
        assert_eq!(tones(&samples, 6), [450, 600, 300, 900, 1050, 60]);
    }

    #[test]
    fn surround_with_a_channel_from_a_stream_it_does_not_have_says_so() {
        // Six channels from four streams, two of them pairs: channels 0 to 5.
        let broken = with_header_changed("surround-5.1.opus", 21, &[9]);
        let Err(why) = Decoded::open(broken.path()) else {
            panic!("it was opened");
        };
        assert_eq!(
            why,
            "Opus in 6 channels with a channel mapping that is not valid"
        );
    }

    #[test]
    fn what_is_not_opus_goes_through_the_common_decoders_and_fails_as_they_say() {
        assert!(matches!(open("tone.m4a"), Decoded::Common { .. }));

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
