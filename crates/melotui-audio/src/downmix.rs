//! Surround made into stereo, for everything after the decoder is stereo.
//!
//! Before this the effects took the first two channels of whatever they were
//! given: a 5.1 file played its front left and front right and nothing else,
//! the centre — where a mix puts the voice — and the surrounds lost.
//!
//! Each channel goes into the left and right as ITU-R BS.775 and ATSC A/52
//! have it, which is what FFmpeg, and mpv and VLC with it, do by default:
//! the front pair as they are, the centre and the surrounds 3 dB down, a rear
//! centre 6 dB down into both, and the LFE left out — the bass in it is also
//! in the main channels of a music mix, and adding it doubles that bass.
//!
//! Nothing is scaled down to make room: the front of a surround track plays
//! as loud as a stereo track does, and the limiter at the end of the effects
//! takes the peaks where the channels add up past full scale. The effects
//! are kept running for a downmixed track for that reason (see
//! [`Downmixed::is_downmixing`]). Players that do scale down — GStreamer,
//! FFmpeg writing integers — leave a 5.1 track 7.7 dB quieter than the
//! stereo ones on either side of it.

use std::f32::consts::FRAC_1_SQRT_2;
use std::time::Duration;

use rodio::{Source, source::SeekError};
use symphonia::core::audio::Channels;

/// Where each speaker goes in stereo, as (left, right).
const GAINS: &[(Channels, f32, f32)] = &[
    (Channels::FRONT_LEFT, 1.0, 0.0),
    (Channels::FRONT_RIGHT, 0.0, 1.0),
    (Channels::FRONT_CENTRE, FRAC_1_SQRT_2, FRAC_1_SQRT_2),
    (Channels::LFE1, 0.0, 0.0),
    (Channels::LFE2, 0.0, 0.0),
    (Channels::REAR_LEFT, FRAC_1_SQRT_2, 0.0),
    (Channels::REAR_RIGHT, 0.0, FRAC_1_SQRT_2),
    (Channels::SIDE_LEFT, FRAC_1_SQRT_2, 0.0),
    (Channels::SIDE_RIGHT, 0.0, FRAC_1_SQRT_2),
    (Channels::REAR_LEFT_CENTRE, FRAC_1_SQRT_2, 0.0),
    (Channels::REAR_RIGHT_CENTRE, 0.0, FRAC_1_SQRT_2),
    (Channels::REAR_CENTRE, 0.5, 0.5),
    // Between the front pair and the centre, or wider than the pair: on
    // their own side, as they are.
    (Channels::FRONT_LEFT_CENTRE, 1.0, 0.0),
    (Channels::FRONT_RIGHT_CENTRE, 0.0, 1.0),
    (Channels::FRONT_LEFT_WIDE, 1.0, 0.0),
    (Channels::FRONT_RIGHT_WIDE, 0.0, 1.0),
    // Overhead, as the speakers they are above: ITU-R BS.2127 puts the
    // front ones where the front pair is.
    (Channels::TOP_FRONT_LEFT, 1.0, 0.0),
    (Channels::TOP_FRONT_RIGHT, 0.0, 1.0),
    (Channels::FRONT_LEFT_HIGH, 1.0, 0.0),
    (Channels::FRONT_RIGHT_HIGH, 0.0, 1.0),
    (Channels::TOP_FRONT_CENTRE, FRAC_1_SQRT_2, FRAC_1_SQRT_2),
    (Channels::FRONT_CENTRE_HIGH, FRAC_1_SQRT_2, FRAC_1_SQRT_2),
    (Channels::TOP_REAR_LEFT, FRAC_1_SQRT_2, 0.0),
    (Channels::TOP_REAR_RIGHT, 0.0, FRAC_1_SQRT_2),
    (Channels::TOP_REAR_CENTRE, 0.5, 0.5),
    (Channels::TOP_CENTRE, 0.5, 0.5),
];

/// Where `speaker` goes in stereo, as (left, right).
pub fn stereo_gains(speaker: Channels) -> (f32, f32) {
    GAINS
        .iter()
        .find(|(of, _, _)| *of == speaker)
        .map_or((0.0, 0.0), |&(_, left, right)| (left, right))
}

/// The speakers of `mask`, in the order their channels are interleaved in:
/// the order of the flags, which is WAVE's, and the order the decoders put
/// them in whatever order the file has them.
pub fn speakers_of(mask: Channels) -> Vec<Channels> {
    mask.iter().collect()
}

/// The speakers a file with `count` channels and nothing to say which has:
/// the layouts FLAC fixes for each count, which Vorbis and WAVE files of that
/// many channels have too.
pub fn usual_speakers(count: usize) -> Option<Vec<Channels>> {
    use Channels as C;
    let mask = match count {
        1 => C::FRONT_CENTRE,
        2 => C::FRONT_LEFT | C::FRONT_RIGHT,
        3 => C::FRONT_LEFT | C::FRONT_RIGHT | C::FRONT_CENTRE,
        4 => C::FRONT_LEFT | C::FRONT_RIGHT | C::REAR_LEFT | C::REAR_RIGHT,
        5 => C::FRONT_LEFT | C::FRONT_RIGHT | C::FRONT_CENTRE | C::REAR_LEFT | C::REAR_RIGHT,
        6 => {
            C::FRONT_LEFT
                | C::FRONT_RIGHT
                | C::FRONT_CENTRE
                | C::LFE1
                | C::REAR_LEFT
                | C::REAR_RIGHT
        }
        7 => {
            C::FRONT_LEFT
                | C::FRONT_RIGHT
                | C::FRONT_CENTRE
                | C::LFE1
                | C::REAR_CENTRE
                | C::SIDE_LEFT
                | C::SIDE_RIGHT
        }
        8 => {
            C::FRONT_LEFT
                | C::FRONT_RIGHT
                | C::FRONT_CENTRE
                | C::LFE1
                | C::REAR_LEFT
                | C::REAR_RIGHT
                | C::SIDE_LEFT
                | C::SIDE_RIGHT
        }
        _ => return None,
    };
    Some(speakers_of(mask))
}

/// How each channel of a frame of `count` goes into stereo, for the
/// `speakers` it is said to have — or, where that is not as many as there
/// are channels, the usual ones for that many. With more channels than any
/// usual layout and nothing to say what they are, the first two are the
/// left and right and the rest are left out, as before.
fn gains_for(count: usize, speakers: Option<&[Channels]>) -> Vec<(f32, f32)> {
    let usual;
    let speakers = match speakers.filter(|speakers| speakers.len() == count) {
        Some(speakers) => speakers,
        None => match usual_speakers(count) {
            Some(speakers) => {
                usual = speakers;
                &usual
            }
            None => {
                let mut gains = vec![(0.0, 0.0); count];
                gains[0] = (1.0, 0.0);
                if count > 1 {
                    gains[1] = (0.0, 1.0);
                }
                return gains;
            }
        },
    };
    if count == 1 {
        // Mono, wherever it is said to be, is the same in both.
        return vec![(1.0, 1.0)];
    }
    speakers
        .iter()
        .map(|&speaker| stereo_gains(speaker))
        .collect()
}

/// `inner` in stereo: as it is if it is mono or stereo, mixed down if it
/// has more channels.
pub struct Downmixed<S> {
    inner: S,
    /// Mixing, and how: one pair of gains for each channel of a frame.
    gains: Option<Vec<(f32, f32)>>,
    /// The channels a frame has, which the gains are for.
    count: usize,
    /// The right half of the frame mixed last, still to be given.
    right: Option<f32>,
}

impl<S: Source<Item = f32>> Downmixed<S> {
    /// `inner`, whose channels are `speakers` if that is known.
    pub fn new(inner: S, speakers: Option<&[Channels]>) -> Self {
        let count = usize::from(inner.channels().get());
        let gains = (count > 2).then(|| gains_for(count, speakers));
        Self {
            inner,
            gains,
            count,
            right: None,
        }
    }

    /// Whether the channels are being mixed down: there may then be peaks
    /// past full scale, for the limiter to take.
    pub fn is_downmixing(&self) -> bool {
        self.gains.is_some()
    }
}

impl<S: Source<Item = f32>> Iterator for Downmixed<S> {
    type Item = f32;

    #[inline]
    fn next(&mut self) -> Option<f32> {
        if self.gains.is_none() {
            return self.inner.next();
        }
        if let Some(right) = self.right.take() {
            return Some(right);
        }
        // A stream can change its channels between spans: what comes now
        // is mixed for what it is, and still comes out in stereo.
        let count = usize::from(self.inner.channels().get());
        if count != self.count {
            self.count = count;
            self.gains = Some(gains_for(count, None));
        }
        let gains = self.gains.as_deref()?;
        let (mut left, mut right) = (0.0, 0.0);
        for &(to_left, to_right) in gains {
            let sample = self.inner.next()?;
            left += sample * to_left;
            right += sample * to_right;
        }
        self.right = Some(right);
        Some(left)
    }
}

impl<S: Source<Item = f32>> Source for Downmixed<S> {
    fn current_span_len(&self) -> Option<usize> {
        let span = self.inner.current_span_len()?;
        if self.gains.is_none() {
            return Some(span);
        }
        // The same frames, two samples each, and the right half of a frame
        // already mixed still to come.
        Some(span / self.count.max(1) * 2 + usize::from(self.right.is_some()))
    }

    fn channels(&self) -> rodio::ChannelCount {
        if self.gains.is_some() {
            rodio::nz!(2)
        } else {
            self.inner.channels()
        }
    }

    fn sample_rate(&self) -> rodio::SampleRate {
        self.inner.sample_rate()
    }

    fn total_duration(&self) -> Option<Duration> {
        self.inner.total_duration()
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), SeekError> {
        self.inner.try_seek(pos)?;
        self.right = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const C: f32 = FRAC_1_SQRT_2;

    /// The gains for a layout of FFmpeg's, by the names it gives them.
    fn gains(names: &str) -> Vec<(f32, f32)> {
        let speakers: Vec<Channels> = names
            .split('+')
            .map(|name| match name {
                "FL" => Channels::FRONT_LEFT,
                "FR" => Channels::FRONT_RIGHT,
                "FC" => Channels::FRONT_CENTRE,
                "LFE" => Channels::LFE1,
                "BL" => Channels::REAR_LEFT,
                "BR" => Channels::REAR_RIGHT,
                "BC" => Channels::REAR_CENTRE,
                "SL" => Channels::SIDE_LEFT,
                "SR" => Channels::SIDE_RIGHT,
                other => panic!("{other}"),
            })
            .collect();
        gains_for(speakers.len(), Some(&speakers))
    }

    fn close(got: &[(f32, f32)], want: &[(f32, f32)]) -> bool {
        got.len() == want.len()
            && got
                .iter()
                .zip(want)
                .all(|(a, b)| (a.0 - b.0).abs() < 1e-6 && (a.1 - b.1).abs() < 1e-6)
    }

    #[test]
    fn each_layout_is_mixed_as_ffmpeg_mixes_it_by_default() {
        // FFmpeg's build_matrix (libswresample/rematrix.c) with its default
        // levels — centre and surround at 1/sqrt(2), LFE at 0 — and no
        // normalising, as for float output.
        for (layout, want) in [
            ("FL+FR+FC", vec![(1.0, 0.0), (0.0, 1.0), (C, C)]),
            ("FL+FR+LFE", vec![(1.0, 0.0), (0.0, 1.0), (0.0, 0.0)]),
            (
                "FL+FR+BL+BR",
                vec![(1.0, 0.0), (0.0, 1.0), (C, 0.0), (0.0, C)],
            ),
            (
                "FL+FR+FC+BC",
                vec![(1.0, 0.0), (0.0, 1.0), (C, C), (0.5, 0.5)],
            ),
            (
                "FL+FR+FC+LFE+BL+BR",
                vec![
                    (1.0, 0.0),
                    (0.0, 1.0),
                    (C, C),
                    (0.0, 0.0),
                    (C, 0.0),
                    (0.0, C),
                ],
            ),
            (
                "FL+FR+FC+LFE+SL+SR",
                vec![
                    (1.0, 0.0),
                    (0.0, 1.0),
                    (C, C),
                    (0.0, 0.0),
                    (C, 0.0),
                    (0.0, C),
                ],
            ),
            (
                "FL+FR+FC+LFE+BC+SL+SR",
                vec![
                    (1.0, 0.0),
                    (0.0, 1.0),
                    (C, C),
                    (0.0, 0.0),
                    (0.5, 0.5),
                    (C, 0.0),
                    (0.0, C),
                ],
            ),
            (
                "FL+FR+FC+LFE+BL+BR+SL+SR",
                vec![
                    (1.0, 0.0),
                    (0.0, 1.0),
                    (C, C),
                    (0.0, 0.0),
                    (C, 0.0),
                    (0.0, C),
                    (C, 0.0),
                    (0.0, C),
                ],
            ),
        ] {
            assert!(
                close(&gains(layout), &want),
                "{layout}: {:?}",
                gains(layout)
            );
        }
    }

    #[test]
    fn every_speaker_there_is_has_somewhere_to_go_and_the_two_sides_mirror() {
        let every = speakers_of(Channels::all());
        assert_eq!(every.len(), GAINS.len(), "a speaker is missing or doubled");
        for speaker in every {
            let (left, right) = stereo_gains(speaker);
            let name = format!("{speaker:?}");
            if name.contains("LEFT") {
                assert!(left > 0.0 && right == 0.0, "{name}");
            } else if name.contains("RIGHT") {
                assert!(right > 0.0 && left == 0.0, "{name}");
            } else {
                assert_eq!(left, right, "{name}");
            }
        }
    }

    #[test]
    fn with_nothing_to_say_which_speakers_the_usual_ones_for_the_count_are_taken() {
        // Said to be the wrong number of speakers, or none: the usual 5.1.
        let usual = gains("FL+FR+FC+LFE+BL+BR");
        assert!(close(&gains_for(6, None), &usual));
        assert!(close(&gains_for(6, Some(&[Channels::FRONT_LEFT])), &usual));
        // More than any usual layout: the first two, as before.
        let ten = gains_for(10, None);
        assert_eq!(ten[..2], [(1.0, 0.0), (0.0, 1.0)]);
        assert!(ten[2..].iter().all(|&g| g == (0.0, 0.0)));
    }

    /// A source of frames given as they are.
    struct Frames {
        samples: std::vec::IntoIter<f32>,
        channels: u16,
    }

    impl Iterator for Frames {
        type Item = f32;
        fn next(&mut self) -> Option<f32> {
            self.samples.next()
        }
    }

    impl Source for Frames {
        fn current_span_len(&self) -> Option<usize> {
            Some(self.samples.len())
        }
        fn channels(&self) -> rodio::ChannelCount {
            rodio::ChannelCount::new(self.channels).unwrap()
        }
        fn sample_rate(&self) -> rodio::SampleRate {
            rodio::nz!(48_000)
        }
        fn total_duration(&self) -> Option<Duration> {
            None
        }
    }

    fn frames(channels: u16, samples: Vec<f32>) -> Frames {
        Frames {
            samples: samples.into_iter(),
            channels,
        }
    }

    #[test]
    fn surround_comes_out_as_stereo_frame_by_frame() {
        let speakers = usual_speakers(6).unwrap();
        // FL FR FC LFE BL BR: a frame with each, and one with the centre alone.
        let source = frames(
            6,
            vec![0.1, 0.2, 0.3, 0.9, 0.4, 0.5, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0],
        );
        let mixed = Downmixed::new(source, Some(&speakers));
        assert!(mixed.is_downmixing());
        assert_eq!(mixed.channels().get(), 2);
        assert_eq!(mixed.current_span_len(), Some(4));
        let out: Vec<f32> = mixed.collect();
        let want = [0.1 + 0.3 * C + 0.4 * C, 0.2 + 0.3 * C + 0.5 * C, C, C];
        assert_eq!(out.len(), want.len());
        for (got, want) in out.iter().zip(want) {
            assert!((got - want).abs() < 1e-6, "{out:?}");
        }
    }

    #[test]
    fn mono_and_stereo_are_left_as_they_are() {
        for channels in [1, 2] {
            let samples = vec![0.25, -0.5, 0.75, 1.0];
            let mixed = Downmixed::new(frames(channels, samples.clone()), None);
            assert!(!mixed.is_downmixing());
            assert_eq!(mixed.channels().get(), channels);
            assert_eq!(mixed.collect::<Vec<_>>(), samples);
        }
    }

    #[test]
    fn a_frame_cut_short_at_the_end_is_not_given_half_mixed() {
        let source = frames(6, vec![0.1, 0.2, 0.3, 0.0, 0.4, 0.5, 0.9, 0.9]);
        let out: Vec<f32> = Downmixed::new(source, None).collect();
        assert_eq!(out.len(), 2, "{out:?}");
    }

    /// How loud each of `tones` is in the left and the right of the file at
    /// `path` as the player mixes it down, decoding and all.
    fn mixed_levels(path: &std::path::Path, tones: &[f64]) -> Vec<(f64, f64)> {
        let decoded = crate::opus::Decoded::open(path).unwrap();
        let speakers = decoded.speakers();
        let mixed = Downmixed::new(decoded, speakers.as_deref());
        assert_eq!(mixed.channels().get(), 2);
        let rate = f64::from(mixed.sample_rate().get());
        let samples: Vec<f32> = mixed.collect();
        let frames = samples.len() / 2;
        let (from, to) = (frames / 4, frames * 3 / 4);
        let level = |side: usize, frequency: f64| {
            let w = std::f64::consts::TAU * frequency / rate;
            let (mut s1, mut s2) = (0f64, 0f64);
            for frame in from..to {
                let s0 = f64::from(samples[frame * 2 + side]) + 2.0 * w.cos() * s1 - s2;
                s2 = s1;
                s1 = s0;
            }
            2.0 * (s1 * s1 + s2 * s2 - 2.0 * w.cos() * s1 * s2)
                .max(0.0)
                .sqrt()
                / (to - from) as f64
        };
        tones.iter().map(|&f| (level(0, f), level(1, f))).collect()
    }

    /// Each speaker's tone in a fixture is at an eighth of full scale.
    const A: f64 = 0.125;
    const AC: f64 = A * std::f64::consts::FRAC_1_SQRT_2;

    fn assert_levels(name: &str, got: &[(f64, f64)], want: &[(f64, f64)], within: f64) {
        for (i, (g, w)) in got.iter().zip(want).enumerate() {
            assert!(
                (g.0 - w.0).abs() < within && (g.1 - w.1).abs() < within,
                "{name}, tone {i}: {g:?}, not {w:?}"
            );
        }
    }

    fn fixture(name: &str) -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    #[test]
    fn surround_files_are_mixed_down_as_ffmpeg_mixes_them() {
        // FL 300, FR 450, FC 600, LFE 60, BL 900, BR 1050 (and SL 1200, SR
        // 1350 for 7.1): each speaker's tone where FFmpeg's `-ac 2` puts it.
        let five_one = [
            (A, 0.0),
            (0.0, A),
            (AC, AC),
            (0.0, 0.0),
            (AC, 0.0),
            (0.0, AC),
        ];
        let tones = [300.0, 450.0, 600.0, 60.0, 900.0, 1050.0];
        // FLAC and Vorbis in their own orders, Opus in Vorbis's in streams.
        // The Vorbis is made at quality 6: below that, Symphonia 0.5's
        // decoder undoes 5.1's channel coupling in the wrong order and the
        // rear right comes out all but silent (fixed in Symphonia by its
        // pull request 604, in no release yet).
        for (name, within) in [
            ("surround-5.1.flac", 0.002),
            ("surround-5.1.ogg", 0.01),
            ("surround-5.1.opus", 0.005),
        ] {
            assert_levels(
                name,
                &mixed_levels(&fixture(name), &tones),
                &five_one,
                within,
            );
        }
        let mut seven_one = five_one.to_vec();
        seven_one.extend([(AC, 0.0), (0.0, AC)]);
        let mut tones = tones.to_vec();
        tones.extend([1200.0, 1350.0]);
        let name = "surround-7.1.opus";
        assert_levels(
            name,
            &mixed_levels(&fixture(name), &tones),
            &seven_one,
            0.005,
        );
    }

    #[test]
    fn a_flac_file_tagged_with_its_speakers_is_mixed_for_those() {
        // Three channels are FL FR FC to FLAC, but this one's tag says the
        // third is the LFE (RFC 9639, section 8.6.2): left out, not centred.
        let got = mixed_levels(&fixture("surround-2.1.flac"), &[300.0, 450.0, 60.0]);
        assert_levels("2.1", &got, &[(A, 0.0), (0.0, A), (0.0, 0.0)], 0.002);
    }

    /// Half a second of a tone for each of `speakers`, in a WAVE file saying
    /// which speakers its channels are for: 300 Hz, 450 Hz, and on up.
    fn wave_file(speakers: Channels) -> tempfile::NamedTempFile {
        let channels = speakers.count() as u16;
        let rate = 16_000u32;
        let frames = rate / 2;
        let mut data = Vec::new();
        for frame in 0..frames {
            for channel in 0..channels {
                let f = 300.0 + 150.0 * f64::from(channel);
                let v = (std::f64::consts::TAU * f * f64::from(frame) / f64::from(rate)).sin() * A;
                data.extend(((v * 32_767.0) as i16).to_le_bytes());
            }
        }
        let block = 2 * channels;
        let mut file = Vec::new();
        file.extend(b"RIFF");
        file.extend((4 + 8 + 40 + 8 + data.len() as u32).to_le_bytes());
        file.extend(b"WAVEfmt ");
        file.extend(40u32.to_le_bytes());
        file.extend(0xFFFEu16.to_le_bytes()); // WAVE_FORMAT_EXTENSIBLE
        file.extend(channels.to_le_bytes());
        file.extend(rate.to_le_bytes());
        file.extend((rate * u32::from(block)).to_le_bytes());
        file.extend(block.to_le_bytes());
        file.extend(16u16.to_le_bytes());
        file.extend(22u16.to_le_bytes());
        file.extend(16u16.to_le_bytes());
        file.extend(speakers.bits().to_le_bytes());
        // KSDATAFORMAT_SUBTYPE_PCM
        file.extend([
            1, 0, 0, 0, 0, 0, 0x10, 0, 0x80, 0, 0, 0xaa, 0, 0x38, 0x9b, 0x71,
        ]);
        file.extend(b"data");
        file.extend((data.len() as u32).to_le_bytes());
        file.extend(data);
        let wave = tempfile::Builder::new().suffix(".wav").tempfile().unwrap();
        std::fs::write(wave.path(), file).unwrap();
        wave
    }

    #[test]
    fn a_wave_file_is_mixed_for_the_speakers_it_says_it_has() {
        use Channels as Ch;
        let tones = [300.0, 450.0, 600.0, 750.0, 900.0, 1050.0];
        // 4.0: a rear centre, into both at half.
        let four = wave_file(Ch::FRONT_LEFT | Ch::FRONT_RIGHT | Ch::FRONT_CENTRE | Ch::REAR_CENTRE);
        let got = mixed_levels(four.path(), &tones[..4]);
        assert_levels(
            "4.0",
            &got,
            &[(A, 0.0), (0.0, A), (AC, AC), (A / 2.0, A / 2.0)],
            0.002,
        );
        // 5.1 with side speakers, where FLAC's 5.1 has rear ones: the same.
        let side = wave_file(
            Ch::FRONT_LEFT
                | Ch::FRONT_RIGHT
                | Ch::FRONT_CENTRE
                | Ch::LFE1
                | Ch::SIDE_LEFT
                | Ch::SIDE_RIGHT,
        );
        let got = mixed_levels(side.path(), &tones);
        let want = [
            (A, 0.0),
            (0.0, A),
            (AC, AC),
            (0.0, 0.0),
            (AC, 0.0),
            (0.0, AC),
        ];
        assert_levels("5.1(side)", &got, &want, 0.002);
    }

    /// Run by hand: `DOWNMIX_IN=file DOWNMIX_OUT=file.f32 cargo test ...
    /// -- --ignored` writes a file as the player mixes it down, decoded and
    /// all, to be compared with what FFmpeg makes of it (`ffmpeg -i file
    /// -ac 2 -f f32le`).
    #[test]
    #[ignore = "writes a file for comparing by hand"]
    fn dump_as_mixed_down() {
        let path = std::env::var("DOWNMIX_IN").unwrap();
        let decoded = crate::opus::Decoded::open(path.as_ref()).unwrap();
        let speakers = decoded.speakers();
        let mixed = Downmixed::new(decoded, speakers.as_deref());
        let bytes: Vec<u8> = mixed.flat_map(f32::to_le_bytes).collect();
        std::fs::write(std::env::var("DOWNMIX_OUT").unwrap(), bytes).unwrap();
    }
}
