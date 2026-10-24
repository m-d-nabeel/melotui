# melotui

`melotui` is a fast, responsive, and feature-rich terminal-based music player written in Rust. It features real-time audio visualization, DSP effects (like Nightcore and Slowed+Reverb), integrated YouTube downloading, and system media controls.

![Screenshot 1](assets/screenshot_1.png)
![Screenshot 2](assets/screenshot_2.png)
![Screenshot 3](assets/screenshot_3.png)

## Core Features

- **Local Library & Playlists**: Automatic scanning and organization of your music files.
- **Audio Playback**: Support for MP3, FLAC, M4A, WAV, OGG, and OPUS.
- **Spectrum Visualizer**: Real-time audio spectrum visualization (supports CAVA).
- **Synced Lyrics**: Time-synced lyrics from `.lrc` files or LRCLIB. Hindi, Urdu and other complex or right-to-left scripts are drawn properly on terminals with image support (kitty, Ghostty, WezTerm, iTerm2, and Sixel terminals such as Windows Terminal, foot and Konsole).
- **DSP Studio**: Real-time audio processing (Speed, Bass, Treble, Reverb, Echo) with a symmetric 8-band EQ.
- **System Integration**: Syncs with Linux/macOS desktop media widgets and supports hardware media keys.
- **Download Support**: Integrated YouTube downloads via `yt-dlp`.

---

## Installation

> [!IMPORTANT]
> **Dependencies are required!**
> Since `melotui` interfaces directly with your system's audio devices and DBUS for media controls, you **must** install the native development libraries before building.

### 1. Install System Dependencies

**Ubuntu / Debian:**

```bash
sudo apt-get update
sudo apt-get install -y libasound2-dev libdbus-1-dev pkg-config
```

**Fedora / RHEL:**

```bash
sudo dnf install alsa-lib-devel dbus-devel pkgconf-pkg-config
```

**macOS:**
*(macOS handles audio and media controls natively via CoreAudio/CoreMedia, so no additional system libraries are required for compilation!)*

### 2. Install yt-dlp (Optional but Recommended)

To use the in-app YouTube download feature (`:dl <query>`), you must have `yt-dlp` installed and available in your system's PATH.

- [yt-dlp installation guide](https://github.com/yt-dlp/yt-dlp/wiki/Installation)

### 3. Build and Run

Make sure you have [Rust installed](https://rustup.rs/). Then, clone the repository and build in release mode:

```bash
git clone https://github.com/m-d-nabeel/melotui.git
cd melotui
cargo build --release
```

The compiled binary will be available at `./target/release/melotui-bin`.

---

## Usage

Start the player:

```bash
./target/release/melotui-bin
```

> [!TIP]
> **Need help?**
> Once inside the app, simply press `?` to open the Help Overlay and see a complete list of all available keybindings and commands!

### Basic Navigation

- Use `h`/`j`/`k`/`l` or the `Arrow Keys` to navigate.
- Press `Tab` to switch between panels.
- Press `:` to enter the command bar (e.g., `:dl lofi hip hop` or `:quit`). It lists every command as you type; `Tab` and `Shift+Tab` complete commands, their options, and playlist and preset names.

### Commands

| Command | What it does |
| --- | --- |
| `:play`, `:pause`, `:stop`, `:next`, `:prev` | Playback |
| `:seek +10`, `:seek 1:30` | Seek by or to a position |
| `:vol 80`, `:vol +10`, `:speed 1.25` | Volume (in percent) and speed |
| `:shuffle on`, `:repeat track` | Shuffle and repeat |
| `:fx bass 2`, `:fx eq 3 -4`, `:fx reset` | Effects, the EQ, and resetting them |
| `:fx slowed`, `:preset load <name>` | Effect macros and presets |
| `:lyrics zoom 3`, `:lyrics zoom panel 2` | Lyrics size |
| `:focus lyrics`, `:vis radial`, `:toggle art` | Layout, visualizer and panels |
| `:pl add <playlist>`, `:artist <query>` | Playlists and search |

### Lyrics size

On terminals that can show images (kitty, Ghostty, WezTerm, iTerm2, and Sixel terminals such as Windows Terminal, foot and Konsole), lyrics are drawn larger than the terminal's text: twice the size in the lyrics panel and three times in the full lyrics view (`:focus lyrics`) by default. Change them for the session with `:lyrics zoom <1-4>`, or for good in `config.toml` (`~/.config/melotui/config.toml` on Linux):

```toml
[ui]
lyrics_zoom = 2.0        # lyrics panel
lyrics_zoom_focus = 3.0  # full lyrics view
```

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
