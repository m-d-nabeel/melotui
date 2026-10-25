//! A song's title and artist, read from what YouTube says about its video.
//!
//! When YouTube knows the song — a release, or a video with the song credited
//! — it says so, and that is used as it stands. Otherwise there is only the
//! video's title, which carries the artist and the song by convention
//! (`Artist - Title`, `Artist「Title」`, `Title | Film | Cast | Label`, …)
//! wrapped in promotion (`(Official Music Video)`, `-MUSiC CLiP-`, …), and
//! the channel's name, which may be the artist's or a label's. Reading the
//! title takes the song out of it; the channel stands in for an artist the
//! title does not name.

/// What YouTube says about a video.
#[derive(Debug, Default, Clone, Copy)]
pub struct Video<'a> {
    pub title: &'a str,
    /// The song, when YouTube knows it.
    pub track: Option<&'a str>,
    /// The song's artists, when YouTube knows them.
    pub artist: Option<&'a str>,
    pub album: Option<&'a str>,
    /// Whose channel it is on.
    pub uploader: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SongName {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
}

/// The song in `video`.
pub fn song_name(video: &Video) -> SongName {
    let known = |s: Option<&str>| s.map(tidy).filter(|s| !s.is_empty());
    let uploader = video.uploader.map(channel_name).filter(|s| !s.is_empty());
    let album = known(video.album);
    if let Some(title) = known(video.track) {
        return SongName {
            title,
            artist: known(video.artist).or(uploader),
            album,
        };
    }
    let (artist, title) = read_title(&tidy(video.title), uploader.as_deref());
    let title = if title.is_empty() {
        tidy(video.title)
    } else {
        title
    };
    SongName {
        title,
        artist: known(video.artist).or(artist).or(uploader),
        album,
    }
}

/// `channel` without what says it is a channel rather than a name.
pub fn channel_name(channel: &str) -> String {
    let mut name = tidy(channel);
    loop {
        let before = name.len();
        for suffix in [
            " - topic",
            "vevo",
            "official youtube channel",
            "official channel",
            "youtube channel",
            "official youtube",
            "official",
        ] {
            if let Some(rest) = strip_suffix_ci(&name, suffix) {
                name = rest.trim_end().to_string();
            }
        }
        if name.len() == before {
            return name;
        }
    }
}

/// The artist, if the title names one, and the song, out of a video title.
fn read_title(title: &str, uploader: Option<&str>) -> (Option<String>, String) {
    // 【Artist】Title — a cover's usual form.
    if let Some(rest) = title.strip_prefix('【')
        && let Some((artist, song)) = rest.split_once('】')
    {
        return (named(artist), finish(song));
    }
    // 「Title」Artist …
    for (open, close) in [('「', '」'), ('『', '』')] {
        if let Some(rest) = title.strip_prefix(open)
            && let Some((song, after)) = rest.split_once(close)
        {
            return (named(&cut_at_noise(after)), finish(song));
        }
    }
    // Artist「Title」… — unless what comes before is itself `A - B`, when
    // the brackets are part of the song or an album named after it.
    // The first bracket: a later one is in a description of the song.
    if let Some((at, open)) = title.char_indices().find(|&(_, c)| c == '「' || c == '『') {
        let close = if open == '「' { '」' } else { '』' };
        let (before, rest) = (&title[..at], &title[at + open.len_utf8()..]);
        if let Some((song, _)) = rest.split_once(close) {
            let artist = before.trim().trim_end_matches(SEPARATORS).trim();
            if !artist.is_empty() && !has_separator(artist) {
                return (named(artist), finish(song));
            }
        }
    }

    // Title | Film | Cast | Label: the song comes first.
    let first = title
        .split('|')
        .map(str::trim)
        .find(|s| !s.is_empty())
        .unwrap_or(title);
    let text = drop_noise_brackets(first);

    // Artist - Title, cut short where promotion begins: in
    // `Title – Lyric Video - Cast`, the cast is not the song.
    let mut parts = Vec::new();
    for part in split_dashes(&text) {
        let kept = strip_noise_end(part);
        let promotion = kept.len() < part.trim().len();
        if !kept.is_empty() {
            parts.push(kept);
        }
        if promotion {
            break;
        }
    }
    // `Title / Translation - …` is not an artist, unless it is the channel.
    if parts.len() >= 2 && parts[0].contains(" / ") && !matches(uploader, &parts[0]) {
        return (None, finish(&parts.join(" - ")));
    }
    if parts.len() >= 2 {
        let (artist, song) = (parts[0].clone(), parts[1..].join(" - "));
        // Title - Artist, sometimes: the channel tells.
        if matches(uploader, &song) && !matches(uploader, &artist) {
            return (named(&song), finish(&artist));
        }
        return (named(&artist), finish(&song));
    }
    let text = parts.pop().unwrap_or_default();

    // Artist / Title, when the artist is whose channel it is: otherwise the
    // slash is as likely to part a title from its translation.
    if let Some((artist, song)) = text.split_once(" / ")
        && matches(uploader, artist)
    {
        return (named(artist), finish(&cut_at_noise(song)));
    }
    // Artist: Title
    if let Some((artist, song)) = text.split_once(": ")
        && artist.split_whitespace().count() <= 4
    {
        return (named(artist), finish(song));
    }
    // Artist 'Title'
    if let Some(stripped) = text.strip_suffix(['\'', '’'])
        && let Some(at) = stripped.rfind([' ', '\u{3000}'])
        && let Some(song) = stripped[at + 1..].strip_prefix(['\'', '‘'])
    {
        return (named(&stripped[..at]), finish(song));
    }
    (None, finish(&text))
}

/// Characters that join an artist to a title.
const SEPARATORS: &[char] = &['-', '–', '—', ':', '|', '/', ' '];

fn has_separator(s: &str) -> bool {
    [" - ", " – ", " — ", " / ", "|"]
        .iter()
        .any(|sep| s.contains(sep))
}

/// `s` split at dashes with spaces either side, as artists and titles are
/// joined: not the hyphen in `Non-Divine` or the one in `-MUSiC CLiP-`.
fn split_dashes(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut rest = s;
    while let Some((at, len)) = [" - ", " – ", " — "]
        .iter()
        .filter_map(|sep| rest.find(sep).map(|at| (at, sep.len())))
        .min()
    {
        parts.push(&rest[..at]);
        rest = &rest[at + len..];
    }
    parts.push(rest);
    parts
}

/// Whether `name` is whose channel it is.
fn matches(uploader: Option<&str>, name: &str) -> bool {
    let squash = |s: &str| -> String {
        s.chars()
            .filter(|c| !c.is_whitespace())
            .flat_map(char::to_lowercase)
            .collect()
    };
    let (Some(uploader), name) = (uploader.map(squash), squash(name)) else {
        return false;
    };
    name.chars().count() >= 2 && (uploader.contains(&name) || name.contains(&uploader))
}

fn named(artist: &str) -> Option<String> {
    let artist = artist.trim().trim_matches(SEPARATORS).trim();
    (!artist.is_empty()).then(|| artist.to_string())
}

/// A song's title, without what is around it.
fn finish(song: &str) -> String {
    let mut song = strip_noise_end(&drop_noise_brackets(song));
    // LIVE「Title」: the brackets hold the song.
    for (open, close) in [('「', '」'), ('『', '』')] {
        if let Some((before, rest)) = song.split_once(open)
            && let Some((inside, after)) = rest.split_once(close)
            && strip_noise_end(after).is_empty()
            && (before.trim().is_empty() || before.trim().eq_ignore_ascii_case("live"))
        {
            song = inside.trim().to_string();
        }
    }
    for (open, close) in [
        ('"', '"'),
        ('“', '”'),
        ('\'', '\''),
        ('‘', '’'),
        ('＂', '＂'),
    ] {
        if let Some(inside) = song.strip_prefix(open).and_then(|s| s.strip_suffix(close)) {
            song = inside.trim().to_string();
        }
    }
    // `Starboy ft. Daft Punk ft. Daft Punk`
    let feats: Vec<&str> = song.split(" ft. ").collect();
    if feats.len() > 2 && feats[feats.len() - 1] == feats[feats.len() - 2] {
        song = feats[..feats.len() - 1].join(" ft. ");
    }
    song
}

/// Words that mark a bracket, or the end of a title, as promotion rather
/// than part of the song.
const NOISE_WORDS: &[&str] = &[
    "official",
    "video",
    "mv",
    "lyric",
    "lyrics",
    "lyrical",
    "audio",
    "visualiser",
    "visualizer",
    "translation",
    "clip",
    "hd",
    "hq",
    "4k",
    "8d",
];
const NOISE_MARKS: &[&str] = &[
    "主題歌",
    "テーマ",
    "アニメ",
    "オープニング",
    "エンディング",
    "劇場版",
    "from anime",
];

fn is_noise(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower
        .split(|c: char| !c.is_alphanumeric())
        .any(|word| NOISE_WORDS.contains(&word))
        || NOISE_MARKS.iter().any(|mark| lower.contains(mark))
}

/// `s` without its bracketed promotion: `(Official Video)`, `[MV]`,
/// `（TVアニメ「…」オープニングテーマ）`; `(feat. …)` and `(Live at …)` stay.
fn drop_noise_brackets(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    'scan: while !rest.is_empty() {
        for (open, close) in [
            ('(', ')'),
            ('[', ']'),
            ('（', '）'),
            ('(', '）'),
            ('（', ')'),
        ] {
            if let Some(inner) = rest.strip_prefix(open)
                && let Some(end) = inner.find(close)
                && is_noise(&inner[..end])
            {
                rest = &inner[end + close.len_utf8()..];
                continue 'scan;
            }
        }
        let c = rest.chars().next().unwrap();
        out.push(c);
        rest = &rest[c.len_utf8()..];
    }
    tidy(&out)
}

/// Phrases that end a title as promotion, longest first so the whole
/// phrase goes.
const NOISE_ENDINGS: &[&str] = &[
    "official music video",
    "official lyric video",
    "official video",
    "official audio",
    "official mv",
    "lyrical video song",
    "lyrical video",
    "lyric video",
    "music video",
    "video song",
    "full video",
    "full song",
    "music clip",
    "lyrical",
    "lyrics",
    "official",
    "mv",
    "歌いました",
    "歌ってみた",
];

/// `s` without promotion at its end: `Official Music Video`, `-MUSiC CLiP-`.
fn strip_noise_end(s: &str) -> String {
    let mut s = s.trim();
    loop {
        let before = s.len();
        s = s.trim_end_matches(SEPARATORS).trim();
        for ending in NOISE_ENDINGS {
            if let Some(rest) = strip_suffix_ci(s, ending)
                && (rest.is_empty()
                    || !ending.is_ascii()
                    || !rest.chars().last().is_some_and(char::is_alphanumeric))
            {
                s = rest.trim();
                break;
            }
        }
        if s.len() == before {
            return s.trim_start_matches(SEPARATORS).trim().to_string();
        }
    }
}

/// `s` up to where promotion starts in it: `SIX LOUNGE Music Video/…`.
fn cut_at_noise(s: &str) -> String {
    let lower = s.to_ascii_lowercase();
    let at = [
        "music video",
        "official",
        "lyric video",
        "music clip",
        " from live",
    ]
    .iter()
    .filter_map(|phrase| lower.find(phrase))
    .min()
    .unwrap_or(s.len());
    s[..at]
        .trim()
        .trim_end_matches(SEPARATORS)
        .trim()
        .to_string()
}

fn strip_suffix_ci<'s>(s: &'s str, suffix: &str) -> Option<&'s str> {
    let at = s.len().checked_sub(suffix.len())?;
    (s.is_char_boundary(at) && s[at..].eq_ignore_ascii_case(suffix)).then(|| &s[..at])
}

/// `s` with full-width punctuation made plain where it does the same job,
/// and its spaces collapsed.
fn tidy(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '｜' => '|',
            '：' => ':',
            '［' => '[',
            '］' => ']',
            '　' => ' ',
            c => c,
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Titles and channels of videos as they came, the song they hold.
    #[test]
    fn songs_are_read_out_of_video_titles() {
        let cases: &[(&str, &str, &str, Option<&str>)] = &[
            // (video title, channel, title, artist)
            (
                "Alan Walker - Faded",
                "Alan Walker",
                "Faded",
                Some("Alan Walker"),
            ),
            (
                "Alan Walker - Faded (8D AUDIO)",
                "8D TUNES",
                "Faded",
                Some("Alan Walker"),
            ),
            (
                "Anuv Jain - Arz Kiya Hai (Official Lyrical Video)",
                "AnuvJainVEVO",
                "Arz Kiya Hai",
                Some("Anuv Jain"),
            ),
            (
                "Armaan Malik: Tootey Khaab (Official Video) | Songster, Kunaal Vermaa | Shabby | Bhushan Kumar",
                "T-Series",
                "Tootey Khaab",
                Some("Armaan Malik"),
            ),
            (
                "Body Thadi | Armaan Malik |Chirayu |  Zaid | Ayaan | Tuba |New Haryanvi Song | 2026",
                "Number 1 Records",
                "Body Thadi",
                Some("Number 1 Records"),
            ),
            (
                "Challa | Full Song | Jab Tak Hai Jaan | Shah Rukh Khan, Katrina Kaif | Rabbi | A. R. Rahman | Gulzar",
                "YRF",
                "Challa",
                Some("YRF"),
            ),
            (
                "Dax - \"Caught You In The Bed\" (Official Music Video)",
                "Dax",
                "Caught You In The Bed",
                Some("Dax"),
            ),
            (
                "JAB TAK Full Video | M.S. DHONI -THE UNTOLD STORY | Armaan Malik, Amaal Mallik |Sushant Singh Rajput",
                "T-Series",
                "JAB TAK",
                Some("T-Series"),
            ),
            (
                "Jab Tak Hai Jaan Title Song | Yash Chopra | Shah Rukh Khan, Katrina, Anushka | A. R. Rahman | Gulzar",
                "YRF",
                "Jab Tak Hai Jaan Title Song",
                Some("YRF"),
            ),
            (
                "King Gnu - AIZO (King Gnu CEN+RAL Tour 2026 at TOKYO ARIAKE ARENA)",
                "King Gnu official YouTube channel   ",
                "AIZO (King Gnu CEN+RAL Tour 2026 at TOKYO ARIAKE ARENA)",
                Some("King Gnu"),
            ),
            (
                "Kuch To Hai Lyrical Video Song | DO LAFZON KI KAHANI | Randeep Hooda, Kajal Aggarwal",
                "T-Series",
                "Kuch To Hai",
                Some("T-Series"),
            ),
            (
                "Lauv - Love U Like That [Official Video]",
                "Lauv",
                "Love U Like That",
                Some("Lauv"),
            ),
            (
                "LiSA『Catch the Moment』（「劇場版 ソードアート・オンライン -オーディナル・スケール-」主題歌）",
                "LiSA Official YouTube",
                "Catch the Moment",
                Some("LiSA"),
            ),
            (
                "LiSA『crossing field』 -MUSiC CLiP-（TVアニメ 「ソードアート・オンライン」 オープニングテーマ）",
                "LiSA Official YouTube",
                "crossing field",
                Some("LiSA"),
            ),
            (
                "LiSA『残酷な夜に輝け』MUSiC CLiP",
                "LiSA Official YouTube",
                "残酷な夜に輝け",
                Some("LiSA"),
            ),
            (
                "MAN WITH A MISSION×milet「絆ノ奇跡」(Live at さいたまスーパーアリーナ）",
                "mwamjapan",
                "絆ノ奇跡",
                Some("MAN WITH A MISSION×milet"),
            ),
            (
                "MIRA - Love Again (Lyric Video)",
                "MIRA",
                "Love Again",
                Some("MIRA"),
            ),
            (
                "MONTAGEM HIKARI",
                "The Vibe Guide",
                "MONTAGEM HIKARI",
                Some("The Vibe Guide"),
            ),
            (
                "MY FIRST STORY × HYDE – “MUGEN” (from Anime “Demon Slayer: Kimetsu no Yaiba”) Official Music Video",
                "Sony Music Japan",
                "MUGEN",
                Some("MY FIRST STORY × HYDE"),
            ),
            (
                "Motoki Ohmori -「催し」Official MV",
                "大森元貴 / Motoki Ohmori",
                "催し",
                Some("Motoki Ohmori"),
            ),
            (
                "Mrs. GREEN APPLE「ライラック」Official Music Video",
                "Mrs. GREEN APPLE",
                "ライラック",
                Some("Mrs. GREEN APPLE"),
            ),
            (
                "Non-Divine - Eyeball (Official Music Video)",
                "Non-Divine",
                "Eyeball",
                Some("Non-Divine"),
            ),
            (
                "Orangestar - Petals (feat. 夏背)",
                "Orangestar",
                "Petals (feat. 夏背)",
                Some("Orangestar"),
            ),
            (
                "SHAUN feat. Conor Maynard - Way Back Home (Lyrics) | Remember when I told you No matter where I go",
                "TheMusicThing",
                "Way Back Home",
                Some("SHAUN feat. Conor Maynard"),
            ),
            (
                "SPYAIR『オレンジ』Music Video（『劇場版ハイキュー!! ゴミ捨て場の決戦』主題歌）",
                "SPYAIR Official YouTube Channel",
                "オレンジ",
                Some("SPYAIR"),
            ),
            (
                "Sajdaa – Lyric Video - Shahrukh Khan, Kajol | My Name is Khan | SEL",
                "SonyMusicIndiaVEVO",
                "Sajdaa",
                Some("SonyMusicIndia"),
            ),
            (
                "Sapna Jahan Full Video - Brothers|Akshay Kumar, Jacqueline|Sonu Nigam, Neeti Mohan",
                "SonyMusicIndiaVEVO",
                "Sapna Jahan",
                Some("SonyMusicIndia"),
            ),
            (
                "Shankar Eshan Loy - Tere Naina Lyric Video - My Name is Khan | Shahrukh Khan | Kajol",
                "SonyMusicIndiaVEVO",
                "Tere Naina",
                Some("Shankar Eshan Loy"),
            ),
            (
                "Sia - The Greatest (Lyrics)",
                "7clouds",
                "The Greatest",
                Some("Sia"),
            ),
            (
                "The 1975 - About You (Official)",
                "The 1975",
                "About You",
                Some("The 1975"),
            ),
            (
                "The Weeknd - Starboy ft. Daft Punk (Official Video) ft. Daft Punk",
                "The Weeknd",
                "Starboy ft. Daft Punk",
                Some("The Weeknd"),
            ),
            (
                "「リカ」SIX LOUNGE Music Video/Illustration by ますだみく",
                "SIX LOUNGE",
                "リカ",
                Some("SIX LOUNGE"),
            ),
            ("【Ado】unravel 歌いました", "Ado", "unravel", Some("Ado")),
            (
                "サカナクション / いらない　-Music Video-",
                "サカナクション sakanaction",
                "いらない",
                Some("サカナクション"),
            ),
            (
                "サカナクション / ミュージック from LIVE Blu-ray/DVD「SAKANAQUARIUM 2024”turn“」",
                "サカナクション sakanaction",
                "ミュージック",
                Some("サカナクション"),
            ),
            (
                "ヨルシカ - LIVE「晴る」",
                "ヨルシカ / n-buna Official",
                "晴る",
                Some("ヨルシカ"),
            ),
            (
                "ヨルシカ - 晴る（OFFICIAL VIDEO）",
                "ヨルシカ / n-buna Official",
                "晴る",
                Some("ヨルシカ"),
            ),
            (
                "ヨルシカ - 茜（OFFCIAL VIDEO）",
                "ヨルシカ / n-buna Official",
                "茜",
                Some("ヨルシカ"),
            ),
            (
                "四月は君の嘘 / Your Lie in April Piano Medley - ピアノメドレー",
                "Fonzi M - Melodies on Piano",
                "四月は君の嘘 / Your Lie in April Piano Medley - ピアノメドレー",
                Some("Fonzi M - Melodies on Piano"),
            ),
            (
                "岡村靖幸・中島健人「瞬発的に恋しよう」Official Music Video",
                "Sony Music Japan",
                "瞬発的に恋しよう",
                Some("岡村靖幸・中島健人"),
            ),
            (
                "風のアンセム feat. suis from ヨルシカ  (Wind's Anthem) - Eve Music Video",
                "Eve",
                "風のアンセム feat. suis from ヨルシカ (Wind's Anthem)",
                Some("Eve"),
            ),
            (
                "소유 (SOYOU) 'Girl' MV",
                "MAGIC STRAWBERRY SOUND, 소유기 SOYOUGI",
                "Girl",
                Some("소유 (SOYOU)"),
            ),
            (
                "Agnes Tachyon Low Cortisol Meme Full Version Lyrics (English Translation)",
                "Layzzz",
                "Agnes Tachyon Low Cortisol Meme Full Version",
                Some("Layzzz"),
            ),
            (
                "ringtone video 10 second",
                "Lx rana00.7 1M",
                "ringtone video 10 second",
                Some("Lx rana00.7 1M"),
            ),
        ];
        let mut wrong = Vec::new();
        for &(video_title, channel, title, artist) in cases {
            let got = song_name(&Video {
                title: video_title,
                uploader: Some(channel),
                ..Video::default()
            });
            if got.title != title || got.artist.as_deref() != artist {
                wrong.push(format!(
                    "{video_title}\n    got  {:?} by {:?}\n    want {title:?} by {artist:?}",
                    got.title, got.artist
                ));
            }
        }
        assert!(wrong.is_empty(), "\n{}", wrong.join("\n"));
    }

    #[test]
    fn what_youtube_knows_of_the_song_comes_first() {
        let got = song_name(&Video {
            title: "Dear (Official Audio)",
            track: Some("Dear"),
            artist: Some("Mrs. GREEN APPLE"),
            album: Some("Dear"),
            uploader: Some("Mrs. GREEN APPLE - Topic"),
        });
        assert_eq!(
            got,
            SongName {
                title: "Dear".into(),
                artist: Some("Mrs. GREEN APPLE".into()),
                album: Some("Dear".into()),
            }
        );
    }

    #[test]
    fn channels_lose_what_marks_them_as_channels() {
        assert_eq!(channel_name("LiSA Official YouTube"), "LiSA");
        assert_eq!(channel_name("SonyMusicIndiaVEVO"), "SonyMusicIndia");
        assert_eq!(channel_name("Mrs. GREEN APPLE - Topic"), "Mrs. GREEN APPLE");
        assert_eq!(
            channel_name("King Gnu official YouTube channel   "),
            "King Gnu"
        );
        assert_eq!(
            channel_name("ヨルシカ / n-buna Official"),
            "ヨルシカ / n-buna"
        );
        assert_eq!(channel_name("YRF"), "YRF");
    }

    #[test]
    fn a_title_with_nothing_to_read_stays_whole() {
        let got = song_name(&Video {
            title: "Official",
            ..Video::default()
        });
        assert_eq!(got.title, "Official");
        assert_eq!(got.artist, None);
    }
}
