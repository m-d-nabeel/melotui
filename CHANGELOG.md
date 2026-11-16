# Changelog

Versions are dates: the year, the month, and a count of the releases in that month.

## Unreleased

- **Opus is played**, in `.opus` files and in `.ogg` files that hold it. It was listed in the library and failed to play.
- Play, pause, volume and the track keys work while lyrics or YouTube results are being picked from.
- `:yt search` uses the browser cookies `download_cookies_from_browser` names, as downloads do.
- [CONFIG.md](CONFIG.md): every setting with its default, how themes are written, and the keymap.

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
