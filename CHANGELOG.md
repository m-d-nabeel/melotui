# Changelog

Versions are dates: the year, the month, and a count of the releases in that month.

## Unreleased

- **melotui is on crates.io**: `cargo install --locked melotui` builds it, and `cargo binstall melotui` fetches the release download. Each of the crates it is made of has a page there saying what it does.
- The `dist` build profile is gone: the release downloads are built with `cargo build --release`, which now strips the binary. The profile's extra optimization made melotui play no lighter, and a release build is what `cargo install` makes anyway.

## 2026.11.1

- **Opus is played**, in `.opus` files and in `.ogg` files that hold it. It was listed in the library and failed to play.
- Play, pause, volume and the track keys work while lyrics or YouTube results are being picked from.
- `:yt search` uses the browser cookies `download_cookies_from_browser` names, as downloads do.
- [CONFIG.md](CONFIG.md): every setting with its default, how themes are written, and the keymap.
- **Surround is mixed down to stereo**, in every format: the centre, where a mix puts the voice, and the surrounds are heard. Only the front left and right were played before. The mix is ITU-R BS.775's, which FFmpeg, mpv and VLC use; the LFE is left out, as they leave it.
- **Opus in up to eight channels** is played, however its channels are shared among its streams. Opus is now decoded by opus-rs: rusty-opus, used before, decodes surround wrongly.
- Opus is played at the gain its header gives, as the Opus spec requires.
- FLAC files tagged with the speakers their channels are for (`WAVEFORMATEXTENSIBLE_CHANNEL_MASK`) are mixed for those speakers.
- The limiter, with any effect on, no longer lets the start of a loud passage through over its ceiling for its first 50 ms.

- **Themes**: fifteen built in (Catppuccin's four, Dracula, Everforest, Gruvbox dark and light, Kanagawa, Nord, One Dark, Rosé Pine, Solarized dark and light, Tokyo Night), each in its project's own colours. **`:theme <name>`** switches while playing and is remembered.
- A theme can be written as a palette of sixteen colours, from which every colour is worked out; a file named for a built-in theme changes that one, and `base` names one to start from. Theme files made before still work, and the colours they leave out come from the theme they are named for.
- Everything is themed: the visualizer, in each of its styles, the background (`[ui] theme_background = false` keeps the terminal's), selections, popups and the DSP studio.
- A theme with a misspelt name or a colour that is not one says so on screen; one that is not there is said at start, not only in the log.
- Opening the DSP studio no longer changes the main screen's visualizer to bars, nor the album art to the visualizer, for good: the studio draws its bars of its own accord.
- Notifications grow to fit what they say, where a long one was cut off after two lines; and what went wrong at start stays in sight once the library has loaded.
- The build profile the release downloads use is now `dist`: `cargo install --profile dist …` or `cargo build --profile dist`, with the binary in `target/dist/`. It was `production`, and the `cargo prod` alias is gone.

Known limits: surround AAC (`.m4a`) cannot be opened, and 5.1 Vorbis made below quality 6 plays with its rear right nearly silent. Both are in the decoder library melotui uses; the Vorbis one is fixed there, in no release yet.

## 2026.11.0

The first release with a date for a version. It follows 0.6.1.

- **Gapless playback**: one track plays straight into the next. `[audio] gapless = false` turns it off.
- **`:yt search <words>`** lists what YouTube Music and YouTube have, in the lyrics panel, to pick a download from. `:yt dl <words or link>` downloads a link or the first match without asking.
- **Downloads are kept as YouTube's AAC (`.m4a`)** as it is sent, in place of being converted to FLAC. `[library] download_cookies_from_browser` lets yt-dlp use a browser's cookies.
- **`z`**, or `:collapse`, closes everything open in the tree.
- Tracks keep their play counts, favorites and playlists when their files are renamed or moved.
- **`:dl` is now `:yt dl`.**
- The library database is migrated when this version first starts.
- A `SHA256SUMS` file is published with the archives.

Earlier releases are described on the [releases page](https://github.com/m-d-nabeel/melotui/releases).
