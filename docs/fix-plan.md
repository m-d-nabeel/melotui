# Fix Workers → Extract Engine → Rethink Snapshot

Three sequential refactors that progressively decouple the architecture while keeping the app functional at every step.

---

## Part 1: Fix Worker Busy-Polling

### Problem

All four workers currently use `try_recv()` + `thread::sleep()` loops. This wastes CPU cycles spinning in userspace when there is no work to do. On a laptop, this means unnecessary battery drain and wakeups.

| Worker | Current Pattern | Sleep | Impact |
|--------|----------------|-------|--------|
| [DB worker](file:///home/m-d-nabeel/Project/melotui/crates/melotui-db/src/worker.rs#L26-L173) | `try_recv()` + sleep | 10ms | ~100 wakeups/sec for idle DB |
| [Download worker](file:///home/m-d-nabeel/Project/melotui/crates/melotui-download/src/worker.rs#L34-L101) | `try_recv()` + sleep | 50ms | ~20 wakeups/sec, plus monitors child proc |
| [Cover worker](file:///home/m-d-nabeel/Project/melotui/crates/melotui-cover/src/worker.rs#L16) | `recv()` blocking | — | ✅ Already correct |
| [Audio worker](file:///home/m-d-nabeel/Project/melotui/crates/melotui-audio/src/worker.rs#L24-L37) | `try_recv()` + sleep | 10ms | Needs ticking for visualizer/progress |

### Strategy

The key insight: **different workers have different cadence needs**.

- **DB worker**: Pure request/response. Zero reason to poll. → **Blocking `recv()`**
- **Cover worker**: Already uses blocking `recv()`. → **No change**
- **Download worker**: Hybrid — must listen for commands AND monitor child processes. → **`recv_timeout()`** with a dual-source select loop
- **Audio worker**: Must tick at ~100Hz for progress/visualizer even when no commands arrive. → **Keep `try_recv()` + sleep**, but the sleep is justified here because there is real periodic work

### Proposed Changes

#### [MODIFY] [worker.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-db/src/worker.rs) — DB Worker

Replace the `loop { try_recv() + sleep }` with blocking `recv()`:

```rust
// Before (lines 26-173):
loop {
    match worker.try_recv() {
        Some(cmd) => { /* handle */ }
        None => { std::thread::sleep(Duration::from_millis(10)); }
    }
}

// After:
while let Some(cmd) = worker.recv() {
    match cmd {
        DatabaseCommand::Shutdown => break,
        cmd => handle_command(&conn, &worker, cmd),
    }
}
```

Extract the giant match body into a `handle_command()` function for readability. The `worker.recv()` call already exists on [`IpcWorker`](file:///home/m-d-nabeel/Project/melotui/crates/melotui-bus/src/channel/mod.rs#L14) and delegates to [`mpsc::Receiver::recv()`](file:///home/m-d-nabeel/Project/melotui/crates/melotui-bus/src/channel/inprocess.rs#L28-L30), which blocks the thread with zero CPU cost until a message arrives.

**CPU impact**: Goes from ~100 wakeups/sec → 0 wakeups/sec when idle. Thread fully parked by the OS.

---

#### [MODIFY] [worker.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-download/src/worker.rs) — Download Worker

This worker is more complex because it must do two things concurrently:

1. Listen for commands (`Request`, `Cancel`, `Shutdown`)
2. Forward progress events from the child `yt-dlp` process

Current approach: single-threaded loop polling both `worker.try_recv()` and `internal_rx.try_recv()` with a 50ms sleep.

**New approach**: Add `recv_timeout()` to the `IpcWorker` trait, then use it to sleep efficiently while still checking for internal events:

```rust
// New method on IpcWorker trait:
fn recv_timeout(&self, timeout: Duration) -> Option<C>;

// Download worker loop:
loop {
    // Block for up to 100ms waiting for a command
    if let Some(cmd) = worker.recv_timeout(Duration::from_millis(100)) {
        match cmd { /* handle */ }
    }

    // Drain internal events (child process progress)
    while let Ok(evt) = internal_rx.try_recv() {
        match evt { /* forward to IPC */ }
    }
}
```

When no download is active, the timeout expiry is harmless (thread wakes, checks nothing, goes back to sleep). When a download IS active, the 100ms timeout ensures progress events are forwarded promptly.

**Alternative considered**: Using `crossbeam-channel::select!` to wait on both channels simultaneously. This would be ideal but adds a dependency and requires changing `WorkerEndpoint` to use crossbeam channels. The `recv_timeout()` approach is simpler and avoids new dependencies.

##### [MODIFY] [mod.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-bus/src/channel/mod.rs)

Add `recv_timeout` to the `IpcWorker` trait:

```rust
pub trait IpcWorker<C>: Send {
    fn recv(&self) -> Option<C>;
    fn try_recv(&self) -> Option<C>;
    fn recv_timeout(&self, timeout: Duration) -> Option<C>;  // NEW
    fn send(&self, evt: IpcEvent);
}
```

##### [MODIFY] [inprocess.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-bus/src/channel/inprocess.rs)

Implement `recv_timeout` on `WorkerEndpoint`:

```rust
fn recv_timeout(&self, timeout: Duration) -> Option<C> {
    self.cmd_rx.recv_timeout(timeout).ok()
}
```

This delegates to [`std::sync::mpsc::Receiver::recv_timeout()`](https://doc.rust-lang.org/std/sync/mpsc/struct.Receiver.html#method.recv_timeout) — no new dependencies needed.

---

#### Audio Worker — No Change

The [audio worker](file:///home/m-d-nabeel/Project/melotui/crates/melotui-audio/src/worker.rs#L24-L37) needs to `tick()` at ~100Hz to:

- Report `PlaybackProgress` events
- Feed the spectrum visualizer ring buffer
- Detect when the sink is empty (`AudioFinished`)

The current `try_recv() + 10ms sleep` is the correct pattern here. A blocking `recv()` would stall progress reporting. A `recv_timeout(10ms)` would work too but offers no real benefit over the current approach since the tick work happens unconditionally.

### Verification

- `cargo build` — no new dependencies required
- `cargo test` — existing `test_bus_channel_communication` test in [inprocess.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-bus/src/channel/inprocess.rs#L43-L78) still passes
- Manual test: launch app, observe CPU usage in `htop` — DB worker thread should show ~0% CPU when idle
- Functional test: scan library, toggle favorites, create/delete playlists, download — all still work

---

## Part 2: Extract `melotui-engine`

### Problem

Currently, the "engine" logic (spawning workers, routing commands, managing the event bus) lives in [melotui-bin](file:///home/m-d-nabeel/Project/melotui/crates/melotui-bin/src/) — the binary crate. This means:

- A future GUI can't reuse the engine without duplicating [ipc_bridge.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-bin/src/ipc_bridge.rs) and [runtime.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-bin/src/runtime.rs)
- The `appmsg_to_ipc` function [requires `&AppState`](file:///home/m-d-nabeel/Project/melotui/crates/melotui-bin/src/ipc_bridge.rs#L95) to read `state.library.root`, coupling the translation layer to UI state
- Testing the engine requires building the entire binary

### Strategy

Create a new `melotui-engine` crate that owns:

1. **Worker lifecycle**: spawning, connecting, and shutting down all workers
2. **Command routing**: the `dispatch()` function that routes `IpcCommand` to the correct worker
3. **Event bus**: the central `mpsc::Receiver<IpcEvent>` that all workers send events to
4. **Message translation**: the `appmsg_to_ipc` and `ipc_to_appmsg` bridge functions

The binary (`melotui-bin`) becomes a thin shell that:

1. Creates an `Engine`
2. Creates a `Tui`
3. Runs the event loop, shuttling messages between engine and TUI

### What Moves Where

```
melotui-bin/src/ipc_bridge.rs  →  melotui-engine/src/lib.rs
                                   (IpcRuntime becomes Engine)
                                   (appmsg_to_ipc, ipc_to_appmsg become engine methods)

melotui-bin/src/runtime.rs     →  stays, but becomes thinner
melotui-bin/src/input_processor.rs → stays (TUI-specific input coalescing)
```

### Proposed Changes

#### [NEW] `crates/melotui-engine/Cargo.toml`

```toml
[package]
name = "melotui-engine"
version = "0.2.0"
edition = "2024"

[dependencies]
melotui-core = { path = "../melotui-core" }
melotui-bus = { path = "../melotui-bus" }
melotui-audio = { path = "../melotui-audio" }
melotui-db = { path = "../melotui-db" }
melotui-download = { path = "../melotui-download" }
melotui-cover = { path = "../melotui-cover" }
tracing = "0.1"
```

#### [NEW] `crates/melotui-engine/src/lib.rs`

The `Engine` struct wraps `IpcRuntime` and provides a clean public API:

```rust
pub struct Engine {
    event_rx: mpsc::Receiver<IpcEvent>,
    audio_tx: mpsc::Sender<AudioCommand>,
    db_tx: mpsc::Sender<DatabaseCommand>,
    download_tx: mpsc::Sender<DownloadCommand>,
    cover_tx: mpsc::Sender<CoverCommand>,
}

impl Engine {
    pub fn new(config: EngineConfig) -> Self { /* spawn workers */ }
    pub fn dispatch(&self, cmd: IpcCommand) { /* route to correct worker */ }
    pub fn poll_events(&self) -> impl Iterator<Item = IpcEvent> + '_ { /* drain event_rx */ }
    pub fn shutdown(&self) { /* send Shutdown to all workers */ }
}
```

#### [NEW] `crates/melotui-engine/src/bridge.rs`

Move `appmsg_to_ipc` and `ipc_to_appmsg` here. Fix the `&AppState` coupling:

```rust
// Before: fn appmsg_to_ipc(msg: &AppMsg, state: &AppState) -> Option<IpcCommand>
// Problem: needs &AppState just for state.library.root

// After: pass the root path explicitly
pub fn appmsg_to_ipc(msg: &AppMsg, library_root: &Path) -> Option<IpcCommand>
```

This removes the dependency on `AppState` entirely. The caller passes `state.library.root` when calling the function.

> [!IMPORTANT]
> **Circular dependency risk**: Currently `melotui-bus` depends on `melotui-core` (for `TrackId`, `SpectrumStyle`, `CoverContext`, `LibrarySnapshot`). And `melotui-engine` would depend on both. This is fine — the dependency graph stays a DAG:
>
> ```
> melotui-core  (domain types, state, reducer)
>       ↑
> melotui-bus   (command/event enums, channel trait)
>       ↑
> melotui-engine (worker lifecycle, routing)
>       ↑
> melotui-bin   (TUI event loop)
> ```

#### [MODIFY] [runtime.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-bin/src/runtime.rs)

Replace `IpcRuntime` usage with `Engine`:

```rust
// Before:
use crate::ipc_bridge::{IpcRuntime, appmsg_to_ipc, ipc_to_appmsg};
let ipc = IpcRuntime::new(producer, db_path.clone());

// After:
use melotui_engine::{Engine, EngineConfig, bridge};
let engine = Engine::new(EngineConfig { db_path, spectrum_producer });
```

#### [DELETE] `crates/melotui-bin/src/ipc_bridge.rs`

Fully replaced by `melotui-engine`.

#### [MODIFY] [Cargo.toml](file:///home/m-d-nabeel/Project/melotui/Cargo.toml)

Add `melotui-engine` to workspace members.

#### [MODIFY] [Cargo.toml](file:///home/m-d-nabeel/Project/melotui/crates/melotui-bin/Cargo.toml)

Replace direct worker crate dependencies with `melotui-engine`:

```toml
# Before:
melotui-audio = { path = "../melotui-audio" }
melotui-db = { path = "../melotui-db" }
melotui-download = { path = "../melotui-download" }
melotui-cover = { path = "../melotui-cover" }

# After:
melotui-engine = { path = "../melotui-engine" }
```

### Verification

- `cargo build` compiles cleanly
- `cargo test -p melotui-engine` — move existing `ipc_bridge` tests here
- `cargo test` — all existing tests pass
- Manual: app launches and behaves identically

---

## Part 3: Rethink `LibrarySnapshot`

### Problem

[`LibrarySnapshot`](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/library/snapshot.rs#L17-L31) is named "snapshot" (implying immutability) but has `&mut self` methods:

- [`toggle_favorite()`](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/library/snapshot.rs#L113)
- [`add_playlist()`](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/library/snapshot.rs#L134)
- [`remove_playlist()`](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/library/snapshot.rs#L144)
- [`add_track_to_playlist()`](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/library/snapshot.rs#L149)
- [`remove_track_from_playlist()`](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/library/snapshot.rs#L159)

This is an **optimistic update** pattern — the UI mutates local state immediately for responsiveness, then sends a command to the DB worker. But it violates the "snapshot" contract and creates these problems:

1. If the DB operation fails, the local state diverges from truth (no rollback)
2. When the library reloads from DB (after scan), any un-persisted mutations are lost
3. The snapshot contains two conceptually different things: **library data** (tracks, folders, indices) and **user edits** (favorites, playlists)

### Strategy: Separate Immutable Data from Mutable Overlay

Split `LibrarySnapshot` into two distinct concepts:

1. **`Library`** (renamed from `LibrarySnapshot`): Immutable data loaded from the database. Tracks, folder tree, artist/album/genre/year indices. **No `&mut self` methods.** Replaced atomically when the DB reloads.

2. **`LibraryOverlay`** (new): A thin mutable layer that records optimistic edits. Lives in `SessionState`. Applied on top of `Library` when querying.

```
┌──────────────────────────────────────────────┐
│  AppState                                    │
│                                              │
│  ┌──────────────┐    ┌───────────────────┐   │
│  │   Library     │    │ SessionState      │   │
│  │ (immutable)   │    │                   │   │
│  │              │    │ ┌───────────────┐  │   │
│  │  tracks      │    │ │LibraryOverlay │  │   │
│  │  artists     │    │ │               │  │   │
│  │  albums      │    │ │ fav_toggles   │  │   │
│  │  genres      │    │ │ added_plists  │  │   │
│  │  years       │    │ │ removed_plists│  │   │
│  │  folder_tree │    │ │ playlist_edits│  │   │
│  │  playlists   │    │ └───────────────┘  │   │
│  └──────────────┘    └───────────────────┘   │
└──────────────────────────────────────────────┘
```

### Why This Is Better (Industry Patterns)

This follows the **CQRS (Command Query Responsibility Segregation)** principle used in Redux, Elm, and event-sourced systems:

- **Queries** read from `Library` + `LibraryOverlay` composed together
- **Commands** mutate only `LibraryOverlay`
- **Sync** replaces `Library` wholesale from DB; `LibraryOverlay` is cleared when the DB confirms success

It also maps to how React/Redux and game engines handle state: the "source of truth" is immutable, and "pending changes" are a separate layer that gets reconciled.

### Proposed Changes

#### [MODIFY] [snapshot.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/library/snapshot.rs) → Rename to `library.rs`

Remove all `&mut self` methods. Rename struct to `Library`:

```rust
pub struct Library {
    pub root: PathBuf,
    pub tracks: HashMap<TrackId, TrackMeta>,
    pub playlists: HashMap<PlaylistId, PlaylistMeta>,
    pub playlist_order: Vec<PlaylistId>,
    pub folder_tree: DirectoryNode,
    pub artists: BTreeMap<String, Vec<TrackId>>,
    pub albums: BTreeMap<String, Vec<TrackId>>,
    pub genres: BTreeMap<String, Vec<TrackId>>,
    pub years: BTreeMap<u32, Vec<TrackId>>,
    pub favorites: Vec<TrackId>,
}

impl Library {
    pub fn new(root, playlists_vec, tracks) -> Self { /* unchanged constructor */ }

    // All methods are &self (read-only)
    pub fn get_track(&self, id: TrackId) -> Option<&TrackMeta> { ... }
    pub fn get_playlist(&self, id: PlaylistId) -> Option<&PlaylistMeta> { ... }
    pub fn resolve_tracks_for_node(&self, node: &SidebarNodeId) -> Vec<&TrackMeta> { ... }
    pub fn find_playlist_by_name(&self, name: &str) -> Option<PlaylistId> { ... }
    // ... other read-only methods stay
}
```

#### [NEW] `crates/melotui-core/src/library/overlay.rs`

```rust
/// Records optimistic mutations that haven't been confirmed by the DB yet.
/// Applied on top of `Library` for UI queries. Cleared on library reload.
#[derive(Debug, Clone, Default)]
pub struct LibraryOverlay {
    /// Tracks whose favorite status was toggled locally
    pub favorite_overrides: HashMap<TrackId, bool>,

    /// Playlists created locally (not yet confirmed by DB with real ID)
    pub added_playlists: Vec<PlaylistMeta>,

    /// Playlists deleted locally
    pub removed_playlists: HashSet<PlaylistId>,

    /// Track additions/removals per playlist
    pub playlist_track_adds: HashMap<PlaylistId, Vec<TrackId>>,
    pub playlist_track_removes: HashMap<PlaylistId, Vec<TrackId>>,
}

impl LibraryOverlay {
    pub fn toggle_favorite(&mut self, id: TrackId, is_favorite: bool) {
        self.favorite_overrides.insert(id, is_favorite);
    }

    pub fn add_playlist(&mut self, id: PlaylistId, name: String) { ... }
    pub fn remove_playlist(&mut self, id: PlaylistId) { ... }
    pub fn add_track_to_playlist(&mut self, pid: PlaylistId, tid: TrackId) { ... }
    pub fn remove_track_from_playlist(&mut self, pid: PlaylistId, tid: TrackId) { ... }

    /// Clear all pending edits (called after DB reload confirms changes)
    pub fn clear(&mut self) { *self = Self::default(); }
}
```

#### [NEW] `crates/melotui-core/src/library/view.rs`

A composing query layer that reads from `Library` + `LibraryOverlay`:

```rust
/// Provides a unified read-only view of library + pending edits.
/// This is what the TUI/GUI renders from.
pub struct LibraryView<'a> {
    library: &'a Library,
    overlay: &'a LibraryOverlay,
}

impl<'a> LibraryView<'a> {
    pub fn new(library: &'a Library, overlay: &'a LibraryOverlay) -> Self { ... }

    pub fn is_favorite(&self, id: TrackId) -> bool {
        // Overlay takes precedence
        if let Some(&fav) = self.overlay.favorite_overrides.get(&id) {
            return fav;
        }
        self.library.get_track(id).map(|t| t.is_favorite).unwrap_or(false)
    }

    pub fn playlists(&self) -> Vec<&PlaylistMeta> {
        // Library playlists minus removed, plus added
        ...
    }

    pub fn favorites(&self) -> Vec<TrackId> {
        // Library favorites + overlay toggles
        ...
    }
}
```

#### [MODIFY] [app_state.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/app_state.rs)

```rust
pub struct AppState {
    // Immutable library data (replaced atomically on DB reload)
    pub library: Library,

    // Session state (includes overlay)
    pub(crate) session: SessionState,
    // ...
}

impl AppState {
    /// Composable view for rendering
    pub fn library_view(&self) -> LibraryView<'_> {
        LibraryView::new(&self.library, &self.session.overlay)
    }
}
```

#### [MODIFY] [session.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/state/session.rs)

Add `overlay` field:

```rust
pub struct SessionState {
    // ... existing fields ...
    pub overlay: LibraryOverlay,
}
```

#### [MODIFY] [library_action.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/action/library_action.rs)

Update all mutating actions to use the overlay instead of mutating `Library` directly:

```rust
// Before:
state.library.toggle_favorite(id, new_status);

// After:
state.session.overlay.toggle_favorite(id, new_status);
```

#### [MODIFY] [runtime.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-bin/src/runtime.rs)

When the library reloads from DB, clear the overlay:

```rust
IpcEvent::LibraryLoaded { snapshot } => {
    state.library = *snapshot;
    state.session.overlay.clear(); // Pending edits now reflected in DB data
}
```

### Verification

- All existing snapshot tests in [snapshot.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/library/snapshot.rs#L367-L535) adapt to the new structure
- New tests for `LibraryOverlay` — toggle, add/remove, clear
- New tests for `LibraryView` — overlay precedence over library
- Manual: toggle favorite → DB reloads → favorite persists; toggle favorite → no reload → favorite visible via overlay

---

## Open Questions

> [!IMPORTANT]
> **Overlay complexity vs. simplicity**: The `LibraryView` compositing layer adds indirection. An alternative simpler approach is to just rename `LibrarySnapshot` to `Library` and accept that it has `&mut self` methods (dropping the "snapshot" pretense). The overlay pattern is more correct but more work. **Which approach do you prefer?**

> [!IMPORTANT]
> **Engine API surface**: Should `Engine` expose typed methods like `engine.play(path)`, `engine.pause()`, `engine.scan()`, or should it stay generic with `engine.dispatch(IpcCommand::Audio(...))`? Typed methods are nicer for external consumers but duplicate the command enum. **Your preference?**

> [!IMPORTANT]
> **Execution order**: I recommend doing these strictly in order (workers → engine → snapshot) because each step builds on the previous one. The worker fix is standalone. The engine extraction will move `ipc_bridge.rs`. The snapshot rethink touches `library_action.rs` which must be updated after the engine move. **Are you okay with this ordering, or do you want to parallelize anything?**

---

## Verification Plan

### Automated Tests

```bash
cargo test --workspace          # All existing tests still pass
cargo test -p melotui-bus       # recv_timeout test
cargo test -p melotui-engine    # Bridge translation tests (moved from melotui-bin)
cargo test -p melotui-core      # Overlay + LibraryView tests
```

### Manual Verification

- Launch app, verify CPU usage in `htop` (DB worker thread ~0% when idle)
- Scan library, toggle favorites, create/delete playlists, download tracks
- Verify all functionality works identically to before
