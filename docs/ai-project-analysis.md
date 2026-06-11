# melotui — Detailed Technical Analysis

## 1. 📊 Codebase Metrics Overview

### 1a. Total Lines of Code by Crate

```text
melotui-core: ~3,821 lines
melotui-tui: ~2,053 lines
melotui-audio: ~896 lines
melotui-bin: ~618 lines
melotui-db: ~659 lines
melotui-ipc: ~399 lines
melotui-download: ~283 lines
melotui-cover: ~201 lines
--------------------------------------------------
Total: ~8,930 lines of Rust
```

### 2. 🏗️ Architectural Issues

#### 2a. Worker Loop Busy-Polling

All worker threads use `try_recv()` + `thread::sleep()` polling loops:

- [Audio worker](file:///home/m-d-nabeel/Project/melotui/crates/melotui-audio/src/worker.rs#L24-L35): `try_recv()` + 10ms sleep
- [Download worker](file:///home/m-d-nabeel/Project/melotui/crates/melotui-download/src/worker.rs#L33-L97): `try_recv()` + 50ms sleep
- [DB worker](file:///home/m-d-nabeel/Project/melotui/crates/melotui-db/src/worker.rs#L27-L173): `try_recv()` + 10ms sleep

**Issue**: This wastes CPU cycles. The audio worker has a legitimate reason (needs to tick visualizer), but the DB and download workers should use blocking `recv()` with a timeout, or a `select!`-style approach.

#### 2c. `LibrarySnapshot` Mutability Contradiction

[LibrarySnapshot](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/library/snapshot.rs#L17-L31) is named "snapshot" (implying immutability) but has multiple `&mut self` methods: `toggle_favorite()`, `add_playlist()`, `remove_playlist()`, `add_track_to_playlist()`, `remove_track_from_playlist()`. This is an **optimistic update** pattern — state is modified locally before the DB confirms — but it violates the snapshot semantics.

#### 2d. `SessionState` Derives `Clone` but Contains `Instant`

[SessionState](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/state/session.rs#L14) derives `Clone`, but contains `Option<std::time::Instant>` (`last_play_time`). `Instant` clones are valid in Rust, but the semantic meaning of cloning a timestamp is questionable — it could lead to bugs if the state is ever serialized or compared.

#### 2e. Tight Coupling in `appmsg_to_ipc`

The [appmsg_to_ipc](file:///home/m-d-nabeel/Project/melotui/crates/melotui-bin/src/ipc_bridge.rs#L95-L196) function requires `&AppState` just for one case (getting `state.library.root` for rescan). This leaks state into what should be a pure message translation layer.

### 3. 🐛 Bug-Prone Patterns

#### 3a. Hardcoded Sample Rate in FFT

[fft.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-audio/src/fft.rs#L49): The FFT uses a **hardcoded `44100` sample rate**:

```rust
samples_fft_to_spectrum(&hann_window, 44100, ...)
```

If the audio source is 48kHz (common) or 96kHz, the spectrum frequency mapping will be incorrect.

#### 3b. `unwrap()` on CAVA stdout/stderr

[download worker](file:///home/m-d-nabeel/Project/melotui/crates/melotui-download/src/worker.rs#L142-L143):

```rust
let stderr = child.stderr.take().unwrap();
let stdout = child.stdout.take().unwrap();
```

If `Stdio::piped()` somehow fails, this panics and takes down the download worker thread.

#### 3c. `partial_cmp().unwrap()` with Floats

[fft.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-audio/src/fft.rs#L102):

```rust
freqs.binary_search_by(|f| f.partial_cmp(&start_f).unwrap())
```

If any frequency value is `NaN`, this panics. Float comparisons should use `total_cmp()` or handle the `None` case.

#### 3d. Race Condition TODO

[playback_action.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/action/playback_action.rs#L187):

```rust
// TODO: clear active_node when playing from queue?
```

[cover_action.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/action/cover_action.rs#L16):

```rust
// TODO: Review - ensure cover matches playing track to avoid race conditions
```

These TODOs indicate known, unresolved edge cases.

### 4. 📊 Code Quality Issues

#### 4a. Verbose Match Arms in `AppState`

[get_active_playlist](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/app_state.rs#L259-L272) and [get_selected_track_id_in_sidebar](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/app_state.rs#L275-L288) explicitly list every `SidebarNodeId` variant returning `None`, instead of using a wildcard `_ => None`. This is fragile — adding a new variant forces changes in multiple places.

#### 4b. Excessive String Cloning

Found **55+ `.clone()` calls** across the codebase, many on `String` and `PathBuf`. Key hotspots:

- `meta.title.clone()` and `meta.file_path.clone()` in playback actions
- `state.library.root.clone()` in IPC bridge
- `name.clone()` in playlist operations

While many are necessary due to ownership, some could be avoided with `&str`/`AsRef<Path>` parameter types or `Arc<str>`.

#### 4c. Inconsistent Error Handling

- `melotui-core` uses `anyhow` but many functions return `Vec<AppMsg>` and silently swallow errors
- Workers use `tracing::error!()` + `IpcEvent::Error { message }` — errors are logged but may not surface to the user
- [Command::parse](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/command.rs#L74-L76): `FromStr::Err` is `()` — no diagnostic information on parse failure

#### 4d. Large Action Files

Several files exceed 300 lines of pure action logic:

- [library_action.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/action/library_action.rs): 496 lines
- [playback_action.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/action/playback_action.rs): 383 lines
- [ui_action.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/action/ui_action.rs): 374 lines

These should be broken down further as they grow.

### 5. 🔒 Security Concerns

#### 5a. Shell Injection in yt-dlp

[download worker](file:///home/m-d-nabeel/Project/melotui/crates/melotui-download/src/worker.rs#L110-L127): The query is passed directly to `yt-dlp` as a command argument. While `Command` API prevents shell injection (arguments are passed directly, not through shell), the `format!("ytsearch1:{}", query)` concatenation means a malicious query string could manipulate yt-dlp's behavior.

#### 5b. No Input Validation on Download Path

The download path is derived from `dirs::audio_dir()` but there's no validation that the download can't write outside the expected directory.

### 6. 📖 Documentation Gaps

- **Zero doc comments** (`///`) on public APIs across all 8 crates (except a handful of one-liners)
- No `CONTRIBUTING.md`, `ARCHITECTURE.md`, or `CHANGELOG.md`
- No inline documentation for complex algorithms (FFT binning, DSP graph topology, tree projection)
- The README describes features well but doesn't cover architecture or contribution guidelines

### 7. 🔧 Configuration Limitations

- **No config file support**: Music directory, keybindings, visualizer preferences, DSP defaults are all hardcoded
- Audio effects have hardcoded ranges (volume ±0.05, speed ±0.1, etc.) — not user-configurable
- Seek offset is hardcoded to 5 seconds ([keybinding.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/keybinding.rs#L98))
- No `.desktop` file or `man` page for Linux installation

### 8. ⚡ Performance Concerns

#### 8a. Full Library Rescan

[scan_directory](file:///home/m-d-nabeel/Project/melotui/crates/melotui-db/src/worker.rs#L236-L254) re-reads metadata from **every audio file** on each scan. It uses `INSERT ... ON CONFLICT DO UPDATE` but doesn't check `file_modified_at` to skip unchanged files. For large libraries (10,000+ tracks), this is unnecessarily slow.

#### 8b. In-Memory Library Duplication

The full library is loaded into memory as a [LibrarySnapshot](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/library/snapshot.rs#L17-L31) containing `HashMap<TrackId, TrackMeta>` plus multiple index maps (artists, albums, genres, years, favorites). For a 20,000-track library, this could consume significant RAM.

#### 8c. Search is O(n)

[collect_matches](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/action/library_action.rs#L442-L495) iterates all tracks/keys for every search, despite having FTS5 full-text search set up in the database. The FTS index is never used for in-app search.

### 9. 🖥️ Platform Support Gaps

- **Linux-centric**: CAVA integration is Linux-only, `libasound2-dev` dependency
- README mentions `%USERPROFILE%\Music` on Windows but actual Windows testing may be limited (CI tests but doesn't run full integration)
- No macOS-specific audio backend considerations (Core Audio)

---

## Technical Debt Summary

| Category | Severity | Count |
|---|---|---|
| Zero tests | 🔴 Critical | Entire codebase |
| Hardcoded FFT sample rate | 🟠 High | 1 occurrence |
| Busy-polling workers | 🟡 Medium | 3 workers |
| Missing config file | 🟡 Medium | System-wide |
| No doc comments | 🟡 Medium | ~150+ public items |
| Excessive cloning | 🟢 Low | ~55 occurrences |
| Verbose match patterns | 🟢 Low | 2 functions |
| Unresolved TODOs | 🟢 Low | 2 items |

---

## Dependencies Analysis

| Dependency | Version | Purpose | Risk |
|---|---|---|---|
| `ratatui` | 0.30 | TUI framework | ✅ Actively maintained |
| `rodio` | 0.21 | Audio playback | ✅ Mature ecosystem |
| `fundsp` | 0.20 | DSP effects | ⚠️ Niche, fewer maintainers |
| `rusqlite` | 0.38 | SQLite (bundled) | ✅ Battle-tested |
| `lofty` | 0.22 | Audio metadata | ✅ Actively maintained |
| `spectrum-analyzer` | 1.7 | FFT analysis | ⚠️ Small project |
| `ratatui-image` | 10.0.4 | Terminal image display | ⚠️ Version churn |
| `image` | 0.25 | Image processing | ✅ Mature |
| `ringbuf` | 0.4 | Lock-free ring buffer | ✅ Stable |

> [!NOTE]
> All dependencies use Rust edition 2024 and stable channel. No nightly features are required.
