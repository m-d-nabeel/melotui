# melotui

melotui is a terminal-based music player written in Rust, featuring audio visualization, library management, and a modular architecture for extensibility.

## Features

- **Audio Playback**: Play music files from your local library with support for MP3, FLAC, M4A, WAV, OGG, and OPUS.
- **Spectrum Visualizer**: Real-time audio spectrum visualization with multiple styles (CAVA, Default, Bar, Particle).
- **DSP Effects**: Real-time audio processing including Speed, Bass, Treble, Reverb, and Echo control.
- **Library Management**: Automatic scanning and organization of music files into directory-based playlists.
- **Terminal UI**: Responsive text-based interface built with Ratatui.
- **IPC Architecture**: Robust separation of concerns using Inter-Process Communication between the UI, Audio Engine, and Downloader.
- **Download Support**: Integrated YouTube download support via `yt-dlp`.
- **Queue Management**: Full queue control (add, remove, play next, clear).
- **Album Art**: Display embedded album art in the visualizer pane.

## Architecture

melotui is built as a workspace with multiple crates:

- `melotui-core`: Core application logic, state management, and reducers.
- `melotui-tui`: Terminal user interface components and rendering.
- `melotui-audio`: Audio analysis, DSP graph (Fundsp), and playback engine (Rodio).
- `melotui-ipc`: Inter-process communication layer utilizing threaded workers.
- `melotui-download`: Background worker for managing downloads.
- `melotui-bin`: Main executable binary and runtime entry point.

## Installation

### Prerequisites

- **Rust**: 1.70 or later
- **Linux Audio**: `libasound2-dev` (e.g., `sudo apt install libasound2-dev` on Ubuntu/Debian)
- **Downloader**: `yt-dlp` (required for download functionality)

### Building from Source

1. Clone the repository:
   ```bash
   git clone [https://github.com/your-username/melotui.git](https://github.com/your-username/melotui.git)
   cd melotui
    ```

2. Build the project:
```bash
cargo build --release

````

3. The binary will be located at `target/release/melotui-bin`.

## Usage

### Running

```bash
./target/release/melotui-bin

```

### Controls

| Category       | Key                         | Action                                           |
| -------------- | --------------------------- | ------------------------------------------------ |
| **General**    | `q`                         | Quit Application                                 |
|                | `?`                         | Toggle Help Overlay                              |
|                | `:`                         | Enter Command Mode                               |
| **Navigation** | `h`/`j`/`k`/`l` or `Arrows` | Navigate Lists                                   |
|                | `Tab`                       | Toggle between Library and Queue panels          |
|                | `Shift`+`Tab`               | Toggle between Spectrum and Album Art            |
| **Playback**   | `Space`                     | Play / Pause                                     |
|                | `Enter`                     | Play Selected Track                              |
|                | `x`                         | Stop                                             |
|                | `,` / `.`                   | Seek -5s / +5s                                   |
|                | `z`                         | Toggle Shuffle                                   |
|                | `c`                         | Cycle Repeat Mode (Off, Single, Track, Playlist) |
| **Queue**      | `a`                         | Add selection to Queue                           |
|                | `Shift`+`A`                 | Play Next (Add to front of Queue)                |
|                | `d`                         | Remove from Queue (when in Queue panel)          |
|                | `D`                         | Clear Queue                                      |
| **Audio FX**   | `+` / `-`                   | Master Volume                                    |
|                | `s` / `S`                   | Speed (Pitch/Tempo) Up / Down                    |
|                | `b` / `B`                   | Bass Boost Up / Down                             |
|                | `t` / `T`                   | Treble Up / Down                                 |
|                | `r` / `R`                   | Reverb Mix Up / Down                             |
|                | `e` / `E`                   | Echo Mix Up / Down                               |

### Command Mode

Press `:` to enter the command bar at the bottom.

- **`:q` / `:quit**`: Exit the application.
- **`:scan` / `:rescan**`: Rescan the library directory.
- **`:dl <query>`**: Search and download a song from YouTube (e.g., `:dl lofi hip hop`).
- **`:v <style>`**: Change visualizer style.
- Options: `cava`, `default`, `bar`, `particle`
- Example: `:v particle`

## Configuration

melotui automatically scans your system's default audio directory (e.g., `~/Music` on Linux/macOS or `%USERPROFILE%\Music` on Windows).

If you have **CAVA** installed on your system, melotui can detect it and use it as the backend for the spectrum visualizer. If not, it falls back to its internal FFT analyzer.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Acknowledgments

- **UI**: [Ratatui](https://github.com/ratatui-org/ratatui)
- **Audio**: [Rodio](https://github.com/RustAudio/rodio) & [Symphonia](https://github.com/pdeljanov/Symphonia)
- **DSP**: [Fundsp](https://github.com/SamiPerttu/fundsp)
- **Metadata**: [Lofty](https://github.com/Serial-ATA/lofty-rs)
- **Image**: [Image-rs](https://github.com/image-rs/image)
