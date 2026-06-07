# melotui TUI — Deep UI Analysis

> **Scope**: All files in `melotui-tui` (~2,100 lines) + UI-related state in `melotui-core`
> **Focus**: Layout, responsiveness, visual consistency, UX friction, and missing features

---

## Current Layout Architecture

```
┌─────────────────────────────────────────────────────┐
│              Terminal (full area)                    │
├────────────┬────────────────────────────────────────┤
│            │                                        │
│  Sidebar   │     Visualizer / Album Art             │
│   22%      │         2/3 height                     │
│            │                                        │
│ (Tree /    ├────────────────────────────────────────┤
│  Queue /   │     Sound Controls (Mixer)             │
│  TrackInfo)│         1/6 height                     │
│            ├────────────────────────────────────────┤
│            │     Playback (Now Playing)             │
│            │         1/6 height                     │
└────────────┴────────────────────────────────────────┘
```

The layout is defined in [view.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/view.rs#L22-L35):

```rust
// Horizontal: 22% sidebar, 78% content
[Constraint::Percentage(22), Constraint::Percentage(78)]

// Vertical right: 2/3 visualizer, 1/3 controls
[Constraint::Ratio(2, 3), Constraint::Ratio(1, 3)]

// Controls split: 50/50
[Constraint::Percentage(50), Constraint::Percentage(50)]
```

---

## Issue Categories

### 🔴 Critical: Layout Breaks at Different Sizes

### 🟠 High: Visual Inconsistency / No Design System

### 🟡 Medium: Widget-Specific Rendering Issues

### 🟢 Low: Missing UX Polish

---

## 🔴 Issue 1: Rigid Percentage-Based Layout Breaks on Small Terminals

**File**: [view.rs:22-35](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/view.rs#L22-L35)

The root layout uses `Constraint::Percentage(22)` for the sidebar. At different terminal widths:

| Terminal Width | Sidebar Width | Usable Sidebar | Problem |
|---|---|---|---|
| 200 cols | 44 cols | ✅ Comfortable | — |
| 120 cols | 26 cols | ⚠️ Tight | Long track names truncated aggressively |
| 80 cols | 17 cols | 🔴 Broken | Tree icons + indent + text barely fit. Only ~8 chars for labels |
| 60 cols | 13 cols | 🔴 Unusable | Icon + indent alone exceed available space |

**Root cause**: No `Min` constraint — at narrow terminals the sidebar shrinks below usable minimum.

**Fix**: Use adaptive constraints:

```rust
// Instead of:
[Constraint::Percentage(22), Constraint::Percentage(78)]

// Use:
[Constraint::Min(25).max(35), Constraint::Fill(1)]
```

Or detect terminal width and collapse sidebar at small sizes.

---

## 🔴 Issue 2: No Minimum Terminal Size Handling

**Problem**: There is no minimum terminal size check anywhere. The app doesn't warn when the terminal is too small, and widgets will overlap, truncate, or panic.

**Where it matters**:

- [track_info.rs:86-100](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/widgets/sidebar/track_info.rs#L86-L100): Fixed `Constraint::Length` values (1+2+1+1+1+1+1+1+1+4 = 14 lines minimum). If the sidebar height is < 14 rows (after art), the info area clips.
- [sound_control_view.rs:33-42](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/widgets/sound_control_view.rs#L33-L42): 3-column layout with `Length(4)` gaps — needs minimum ~30 cols to render sliders.
- [help_view.rs:47-55](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/widgets/help_view.rs#L47-L55): 4-column layout at 25% each — at 80 cols, each column is ~18 chars, truncating keybinding descriptions.

**Fix**: Add a minimum size check in `runtime.rs` before drawing:

```rust
if w < 80 || h < 24 {
    // Render a "Terminal too small" message instead
}
```

---

## 🔴 Issue 3: Bottom Panel Height Starvation

**File**: [view.rs:27-35](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/view.rs#L27-L35)

The bottom 1/3 is split 50/50 between Mixer and Playback. At common terminal heights:

| Terminal Height | Bottom 1/3 | Per Panel | Problem |
|---|---|---|---|
| 50 rows | 16 rows | 8 rows | ✅ OK |
| 35 rows | 11 rows | 5 rows | ⚠️ Mixer sliders cramped — only 1 row per slider |
| 24 rows | 8 rows | 4 rows | 🔴 Broken — playback row content doesn't fit (needs 5 rows: title, spacer, progress, spacer, next) |

**Fix**: Use minimum heights with priority:

```rust
// Playback bar is essential — give it priority
[Constraint::Min(7), Constraint::Length(7)]
// Let visualizer absorb remaining space
```

---

## 🟠 Issue 4: No Centralized Design System / Color Palette

**Problem**: Colors are scattered across **11 different files** with no shared definitions. Each widget defines its own constants inline.

**Color definition locations**:

| File | Constants Defined |
|---|---|
| [tree.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/widgets/sidebar/tree.rs#L19-L32) | `ACTIVE_BORDER`, `INACTIVE_BORDER`, `SELECTED_BG`, `SELECTED_FG`, `PLAYING_TRACK_COLOR`, `ANCESTOR_COLOR`, `FOLDER_ICON_COLOR`, `PLAYLIST_ICON_COLOR`, `CATEGORY_COLOR` |
| [queue.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/widgets/sidebar/queue.rs#L12-L15) | `ACTIVE_BORDER`, `INACTIVE_BORDER`, `SELECTED_BG`, `PLAYING_TRACK_COLOR` (duplicated!) |
| [help_view.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/widgets/help_view.rs#L12-L18) | `OVERLAY`, `TEXT`, `SUBTEXT`, `ACCENT`, `KEY_FG`, `HEADER_FG`, `BORDER` |
| [sound_control_view.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/widgets/sound_control_view.rs#L76-L135) | Inline `Color::Green`, `Color::Blue`, `Color::Yellow`, etc. |
| [playback_view/utils.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/widgets/playback_view/utils.rs#L7-L9) | Inline `Color::Green`, `Color::Yellow`, `Color::Red` |
| [notification_popup.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/widgets/notification_popup.rs#L23-L26) | Inline `Color::Cyan`, `Color::Blue`, `Color::Yellow`, `Color::Red` |
| [track_info.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/widgets/sidebar/track_info.rs) | Inline `Color::Cyan`, `Color::DarkGray`, etc. |
| [spectrum_visualizer_view.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/widgets/visualizer_view/spectrum_visualizer_view.rs#L359-L362) | Mixed `Color::LightBlue`, `Color::Blue`, `Color::DarkGray`, `Color::Gray` |

**Inconsistencies found**:

- `ACTIVE_BORDER = Color::Cyan` is defined **identically** in `tree.rs` and `queue.rs` — copy-paste duplication
- `SELECTED_BG = Color::Rgb(40, 44, 52)` is duplicated in `tree.rs` and `queue.rs`
- `PLAYING_TRACK_COLOR` is `Rgb(166, 227, 161)` (Catppuccin Green) in sidebar but playback uses generic `Color::Green` — they render differently
- Help view uses Catppuccin-inspired `Rgb` values, while other widgets use bare `Color::` enums
- Mixer slider labels use `Color::DarkGray`, but playback uses `Color::DarkGray` too — visually same but semantically unrelated
- Track info badges use `Color::Green` for lossless and `Color::Blue` for lossy — but playback status uses the same `Color::Green` for "playing"

**Fix**: Create `theme.rs` module:

```rust
pub struct Theme {
    pub border_active: Color,
    pub border_inactive: Color,
    pub bg_selected: Color,
    pub fg_selected: Color,
    pub fg_default: Color,
    pub fg_muted: Color,
    pub accent_primary: Color,
    pub accent_playing: Color,
    pub accent_warning: Color,
    pub accent_error: Color,
    // ... semantic colors
}
```

---

## 🟠 Issue 5: Mixed Color Systems

**Problem**: The codebase mixes three different color systems:

1. **Named colors**: `Color::Green`, `Color::Cyan`, `Color::DarkGray` — terminal-dependent, look different on every terminal
2. **RGB colors**: `Color::Rgb(166, 227, 161)` — consistent across terminals (if truecolor supported)
3. **Computed colors**: Dynamic `Color::Rgb(r, g, b)` in visualizers — always consistent

Widgets that use named colors will look different in:

- Light terminal themes (where `Color::DarkGray` becomes light)
- Terminals without 256-color support
- tmux without `set -g default-terminal "tmux-256color"`

The sidebar uses RGB but the playback panel uses named colors. This creates visual dissonance.

**Fix**: Standardize on `Color::Rgb(...)` everywhere for consistency, or provide fallback named colors for limited terminals.

---

## 🟠 Issue 6: Branding Art Overflows at Narrow Widths

**File**: [album_art_view.rs:12-20](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/widgets/visualizer_view/album_art_view.rs#L12-L20)

The `BRANDING_ART` ASCII art is **~92 characters wide**. At a 120-col terminal with 22% sidebar, the content area is ~94 cols — the art barely fits. At 100 cols, the art area is ~78 cols and the art **overflows and wraps**, creating a garbled mess.

```
░▒▓██████████████▓▒░░▒▓████████▓▒░▒▓█▓▒░      ░▒▓██████▓▒░▒▓████████▓▒░▒▓█▓▒░░▒▓█▓▒░▒▓█▓▒░
```

This line is 92 chars wide. No wrapping or size-based fallback exists.

**Fix**: Either add a smaller ASCII art variant or use a simple centered text logo when the area is too narrow.

---

## 🟡 Issue 7: Track Info Layout is Fragile

**File**: [track_info.rs:86-100](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/widgets/sidebar/track_info.rs#L86-L100)

The info area uses **10 fixed-height constraints**:

```rust
Constraint::Length(1), // Spacer
Constraint::Length(2), // Title
Constraint::Length(1), // Artist
Constraint::Length(1), // Album
Constraint::Length(1), // Spacer
Constraint::Length(1), // Badges
Constraint::Length(1), // Details
Constraint::Length(1), // Spacer
Constraint::Length(1), // Separator
Constraint::Min(4),    // File Info
```

Total minimum: **14 rows** for info + album art (which is `min(15, 40%)` height). On a 30-row terminal with 22% sidebar, the sidebar area is ~30 rows high. With art at 15 rows, only 15 rows remain for info — that's barely enough. On a 24-row terminal, art gets 12 rows, leaving only 12 for info which clips the file info section.

Additionally, the title gets `Length(2)` but is rendered as a single `Paragraph` — if the title is short, there's a wasted row.

---

## 🟡 Issue 8: Notification Popup Positioning Bug

**File**: [notification_popup.rs:85-101](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/widgets/notification_popup.rs#L85-L101)

The `top_right_rect` function calculates:

```rust
let y = (area.y)
    .saturating_sub(effective_height)
    .saturating_sub(margin);
```

`area.y` is typically `0` for full screen. `saturating_sub(4)` on `0` = `0`. Then `saturating_sub(1)` = `0`. So `y = 0`, and the popup renders at the **top-left area origin** — but the intent is top-right. The `y` calculation should be `area.y + margin`, not `area.y - height - margin`.

The notification popup may overlap the sidebar title "Library" or be visually confusing when it renders at `y=0`.

---

## 🟡 Issue 9: Marquee Text Uses Byte Indexing, Not Char Indexing

**File**: [sidebar/mod.rs:21-37](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/widgets/sidebar/mod.rs#L21-L37)

```rust
if text.len() <= max_width {
    return text.to_string();
}
```

`text.len()` returns **byte count**, but `max_width` is meant to represent **display columns**. For non-ASCII characters (common in music metadata — accented characters, CJK, emoji):

- `"Café"` — 5 bytes, 4 display chars
- `"日本語の曲"` — 15 bytes, 10 display chars (double-width)

The truncation in `tree.rs:194` is even worse:

```rust
&row.label[0..available_text_width.saturating_sub(1) as usize]
```

This slices **bytes**, which will panic on multi-byte UTF-8 sequences.

**Fix**: Use `unicode_width::UnicodeWidthStr` for display width, and `char` boundaries for truncation.

---

## 🟡 Issue 10: No Visual Feedback for Active Panel in Playback/Mixer

**Problem**: The sidebar clearly shows active/inactive state with different border colors (`Cyan` vs `Rgb(60,60,60)`). But the Mixer and Playback panels have **no active/inactive state** — they always look the same regardless of focus.

- [sound_control_view.rs:26](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/widgets/sound_control_view.rs#L26): Always `Color::LightBlue` border
- [playback_view/mod.rs:18-23](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/widgets/playback_view/mod.rs#L18-L23): No border color at all

Users can't tell which panel accepts keyboard input when they Tab between panels.

---

## 🟡 Issue 11: Help View Columns Clip at Normal Widths

**File**: [help_view.rs:47-55](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/widgets/help_view.rs#L47-L55)

The help view uses 4 equal-width columns at `Percentage(25)` each. At a typical 120-col terminal with 90% width overlay:

- Overlay width: 108 cols
- Per column: ~27 cols
- After borders + padding: ~23 usable chars

The command descriptions like `:pl create <n>  New Playlist` need ~30+ chars. With `Wrap { trim: true }`, they wrap awkwardly mid-word, making the help view hard to read.

The bindings are sorted alphabetically, not by logical grouping — "Move Down" appears before "Navigate" which appears before "Play Selected".

---

## 🟡 Issue 12: Progress Bar Capsule Theme Has a Visual Bug

**File**: [progress_bar_view.rs:27-38](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/widgets/playback_view/progress_bar_view.rs#L27-L38)

```rust
ProgressBarTheme::Capsule => {
    let body = if filled_count > 0 {
        if filled_count == 1 {
            "●".to_string()
        } else {
            let body_len = filled_count.saturating_sub(2);
            format!("{}", "█".repeat(body_len))
        }
    };
```

When `filled_count == 2`: `body_len = 0`, so the filled portion is an empty string, but `filled_count > 0` so it tries to render. Result: the bar "jumps" from nothing to a visible bar at ~3% progress, with a dead zone between 0-2%.

Also, the Capsule theme was supposed to have rounded ends but only uses `●` for single-char fills and flat `█` for the body — there's no visual difference from Retro theme at larger widths.

---

## 🟡 Issue 13: Sound Control Sliders Have Dead Zones

**File**: [sound_control_view.rs:173](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/widgets/sound_control_view.rs#L173)

```rust
if config.label == "Speed" || config.label == "Bass" || config.label == "Treble" {
    format!("x{:.1}", config.val)
} else {
    format!("{:.0}%", config.val * 100.0)
}
```

Value formatting is based on **string comparison** of the label name — fragile and will break if labels change. Should use an enum or format callback.

Additionally, all sliders share the exact same visual appearance — you can't quickly distinguish Volume from Bass from Reverb without reading the label. Color is the only differentiator, but labels are `DarkGray` which is nearly invisible on dark terminals.

---

## 🟡 Issue 14: Image Cache Resize on Every Frame

**File**: [image_cache.rs:31-78](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/image_cache.rs#L31-L78)

`update_protocol()` checks if `target_w == self.width && target_h == self.height`, but `update()` at [line 155-158](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/image_cache.rs#L155-L158) calls `picker.new_resize_protocol(img.as_ref().clone())` which clones the entire `DynamicImage` and recreates the protocol on every source change. This is expensive (image resize + clone) and causes a visible frame drop when switching tracks.

The `update_protocol` does size-based caching correctly, but `update_state` bypasses it by directly setting `self.protocol` with the raw (un-resized) image on source change. The image then gets properly resized on the next `update_protocol` call, causing **double processing**.

---

## 🟢 Issue 15: No Status Bar / Footer

**Problem**: There's no persistent status bar showing:

- Current panel focus (Library / Queue / TrackInfo)
- Search query when active
- Track count / queue length
- Keybinding hints (`? Help  : Command  Tab Panel`)

Users have to memorize keybindings or press `?` to see the help view. A minimal status bar would dramatically improve discoverability.

---

## 🟢 Issue 16: No Loading / Empty States

**Problem**: When the library loads, `state.set_notification(NotificationKind::Info, "Loading library...")` is shown as a small popup, but the main sidebar area is just empty with no visual indication.

Similarly, when the visualizer has no spectrum data (`spectrum.is_empty()`), the widget just returns early and renders nothing — a blank bordered box with "Visualizer" title.

Better: Show a centered "Loading...", "No tracks found", or idle animation.

---

## 🟢 Issue 17: No Keyboard Shortcut to Resize Panels

**Problem**: The sidebar is fixed at 22%. Users can't resize it to see longer track names, or collapse it for more visualizer space. Power users (especially on wide monitors) want panel resize.

---

## 🟢 Issue 18: Overlay Dim Effect is Inconsistent

**File**: [view.rs:44-89](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/view.rs#L44-L89)

Every overlay (Command, PlaylistPicker, Confirm, Help) renders a dim background with:

```rust
frame.render_widget(
    Block::default().style(Style::default().add_modifier(Modifier::DIM)),
    area,
);
```

But `Modifier::DIM` only modifies **text already rendered** — it doesn't create a proper dimming effect on all terminals. Some terminals ignore DIM entirely. The visual effect is inconsistent: sometimes the background dims, sometimes nothing visible changes.

Better: Use a semi-transparent background color like `Color::Rgb(0, 0, 0)` with a custom widget that overwrites all cells with a dark background.

---

## 🟢 Issue 19: Confirm Dialog Has No Border or Clear Boundary

**File**: [confirm_view.rs](file:///home/m-d-nabeel/Project/melotui/crates/melotui-tui/src/widgets/confirm_view.rs)

The confirm dialog is a single-row text at the bottom of the screen with `bg(Color::Rgb(25, 25, 30))`. It has no border, no block, and no visual separation from the content above. On dark terminals, it's nearly invisible — the user might not notice the prompt.

---

## 🟢 Issue 20: No Scroll Indicator in Help View

The help view can have more bindings than fit vertically (especially with custom bindings in the future), but there's no scroll support — content is truncated silently.

---

## 🟠 Issue 21: No Focus / Zen Mode (NEW — User Priority)

> [!IMPORTANT]
> **User-requested feature**: The ability to isolate a single widget and hide everything else. This is a must-have for the TUI UX.

### Problem

Currently the layout is **rigid and all-or-nothing** — the sidebar, visualizer, mixer, and playback bar are always visible. Users can't:

- Maximize the visualizer for an immersive music experience
- Show just the playback bar as a minimal "now playing" widget
- View album art fullscreen
- Hide the mixer when they don't need it

### Proposed Design: `LayoutMode` Enum

Add a new state to `melotui-core`:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum LayoutMode {
    #[default]
    Full,                    // Current default: sidebar + visualizer + mixer + playback
    Zen(ZenTarget),          // Single widget fullscreen
    Compact,                 // Visualizer + playback only (no sidebar, no mixer)
    Minimal,                 // Playback bar only (tiny footprint)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ZenTarget {
    Visualizer,              // Fullscreen visualizer/spectrum
    AlbumArt,                // Fullscreen album art
    Library,                 // Fullscreen library tree
    Queue,                   // Fullscreen queue
}
```

### Layout Diagrams for Each Mode

#### `Full` (current default)

```
┌──────────┬──────────────────────────────────┐
│          │       Visualizer / Art           │
│ Sidebar  ├──────────────────────────────────┤
│          │       Mixer                      │
│          ├──────────────────────────────────┤
│          │       Playback                   │
└──────────┴──────────────────────────────────┘
```

#### `Zen(Visualizer)` — Immersive visualizer mode

```
┌─────────────────────────────────────────────┐
│                                             │
│             Visualizer / Spectrum           │
│               (fullscreen)                  │
│                                             │
├─────────────────────────────────────────────┤
│  ▶ Song Title • Artist     03:21 / 04:15   │
└─────────────────────────────────────────────┘
```

A thin 1-2 row playback overlay at the bottom. Visualizer fills the rest. The overlay can auto-hide after a few seconds of no input (like a video player).

#### `Zen(AlbumArt)` — Fullscreen album art

```
┌─────────────────────────────────────────────┐
│                                             │
│              Album Art                      │
│            (fullscreen)                     │
│                                             │
├─────────────────────────────────────────────┤
│  ▶ Song Title • Artist     03:21 / 04:15   │
└─────────────────────────────────────────────┘
```

#### `Zen(Library)` — Fullscreen library browser

```
┌─────────────────────────────────────────────┐
│                                             │
│           Library Tree (full width)         │
│             Much more space for             │
│           track names and metadata          │
│                                             │
├─────────────────────────────────────────────┤
│  ▶ Song Title • Artist     03:21 / 04:15   │
└─────────────────────────────────────────────┘
```

#### `Compact` — No sidebar, no mixer

```
┌─────────────────────────────────────────────┐
│                                             │
│         Visualizer / Album Art              │
│                                             │
├─────────────────────────────────────────────┤
│  ▶ Song Title • Artist   ═══●═══ 03:21     │
│  Up Next: Next Song                         │
└─────────────────────────────────────────────┘
```

#### `Minimal` — Just a playback bar

```
┌─────────────────────────────────────────────┐
│  ▶ Song Title • Artist   ═══●═══ 03:21     │
│  Vol: ██●── 80%  Shuf: ✓  Rep: 🔁          │
│  Up Next: Next Song                         │
└─────────────────────────────────────────────┘
```

3-row compact player. Could even be used in a floating terminal widget.

### Keybinding Scheme

| Key | Action | Description |
|---|---|---|
| `F` or `f` | Cycle layout forward | Full → Compact → Minimal → Full |
| `Shift+F` | Cycle layout backward | Full → Minimal → Compact → Full |
| `z` | Toggle zen mode | Normal ↔ Zen(current panel) |
| `1` | Zen: Visualizer | Jump directly to fullscreen visualizer |
| `2` | Zen: Album Art | Jump directly to fullscreen album art |
| `3` | Zen: Library | Jump directly to fullscreen library |
| `4` | Zen: Queue | Jump directly to fullscreen queue |
| `Esc` | Exit zen/compact/minimal | Return to Full mode |

### State Management

Add to [UIState](file:///home/m-d-nabeel/Project/melotui/crates/melotui-core/src/state/ui.rs#L27-L38):

```rust
pub struct UIState {
    // ... existing fields ...
    pub layout_mode: LayoutMode,
    pub zen_show_controls: bool,      // auto-hide playback overlay in zen
    pub zen_controls_timer: f64,      // seconds since last keypress
}
```

### Implementation in view.rs

The `draw()` function would branch on `state.layout_mode()`:

```rust
pub fn draw(frame: &mut Frame, state: &AppState, image_cache: &mut ImageCache) {
    let area = frame.area();

    match state.layout_mode() {
        LayoutMode::Full => draw_full(frame, area, state, image_cache),
        LayoutMode::Zen(target) => draw_zen(frame, area, state, image_cache, target),
        LayoutMode::Compact => draw_compact(frame, area, state, image_cache),
        LayoutMode::Minimal => draw_minimal(frame, area, state),
    }

    // Overlays always render on top
    notification_popup::draw(frame, area, state);
    draw_overlays(frame, area, state);
}

fn draw_zen(frame: &mut Frame, area: Rect, state: &AppState,
            cache: &mut ImageCache, target: &ZenTarget) {
    let show_controls = state.zen_show_controls();

    let (main_area, controls_area) = if show_controls {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Fill(1), Constraint::Length(2)])
            .split(area);
        (chunks[0], Some(chunks[1]))
    } else {
        (area, None)
    };

    // Render the focused widget fullscreen
    match target {
        ZenTarget::Visualizer => visualizer_view::draw(frame, main_area, state, cache),
        ZenTarget::AlbumArt => album_art_view::draw_fullscreen(frame, main_area, state, cache),
        ZenTarget::Library => tree::draw(frame, main_area, state),
        ZenTarget::Queue => queue::draw(frame, main_area, state),
    }

    // Render minimal playback overlay
    if let Some(ctrl) = controls_area {
        draw_zen_playback_overlay(frame, ctrl, state);
    }
}
```

### Zen Playback Overlay Widget

A new minimal widget (`zen_overlay.rs`):

```
┌─────────────────────────────────────────────────┐
│ ▶ Song Title • Artist       ━━━━●━━━ 3:21/4:15 │
└─────────────────────────────────────────────────┘
```

Single row: status icon, title, artist, progress bar, timestamps — all inline. No borders, just a dark background strip.

### Auto-Hide Behavior

In zen mode, the playback overlay auto-hides after 3 seconds of no input:

1. Any keypress resets `zen_controls_timer = 0.0`
2. Each tick increments `zen_controls_timer += dt`
3. When `zen_controls_timer > 3.0`, set `zen_show_controls = false`
4. Any keypress sets `zen_show_controls = true` and resets the timer

This mimics video player HUD behavior.

### Command Integration

Also accessible via the command bar:

```
:layout full       → Full mode
:layout compact    → Compact mode
:layout minimal    → Minimal mode
:layout zen        → Zen mode (current panel)
:zen visualizer    → Zen: Visualizer
:zen art           → Zen: Album Art
:zen library       → Zen: Library
:zen queue         → Zen: Queue
```

### Files to Create/Modify

| File | Action |
|---|---|
| `melotui-core/src/state/ui.rs` | Add `LayoutMode`, `ZenTarget` enums |
| `melotui-core/src/state/ui.rs` | Add `layout_mode`, `zen_show_controls`, `zen_controls_timer` to `UIState` |
| `melotui-core/src/message/ui.rs` | Add `SetLayoutMode`, `ToggleZen`, `ZenFocus` messages |
| `melotui-core/src/action/ui_action.rs` | Add handlers for layout mode changes |
| `melotui-core/src/keybinding.rs` | Add `f`, `F`, `z`, `1-4` bindings |
| `melotui-core/src/command.rs` | Add `:layout` and `:zen` commands |
| `melotui-tui/src/view.rs` | Branch `draw()` on layout mode |
| `melotui-tui/src/widgets/zen_overlay.rs` | **[NEW]** Minimal playback overlay widget |
| `melotui-tui/src/widgets/mod.rs` | Register `zen_overlay` module |
| `melotui-bin/src/runtime.rs` | Tick `zen_controls_timer` in the render loop |

**Effort**: ~3-4 days

---

## Summary Table

| # | Issue | Severity | Widget | LOC Impact |
|---|---|---|---|---|
| 1 | Rigid percentage layout breaks on small terminals | 🔴 Critical | view.rs | ~5 lines |
| 2 | No minimum terminal size handling | 🔴 Critical | runtime.rs | ~15 lines |
| 3 | Bottom panel height starvation | 🔴 Critical | view.rs | ~5 lines |
| 4 | No centralized color/theme system | 🟠 High | All widgets | New file ~100 lines |
| 5 | Mixed named vs RGB color systems | 🟠 High | All widgets | ~80 edits |
| 6 | Branding art overflow on narrow terminals | 🟠 High | album_art_view.rs | ~10 lines |
| **21** | **No Focus / Zen Mode** | **🟠 High** | **view.rs + new files** | **~250 lines** |
| 7 | Track info fragile fixed-height layout | 🟡 Medium | track_info.rs | ~15 lines |
| 8 | Notification popup y-position bug | 🟡 Medium | notification_popup.rs | ~3 lines |
| 9 | Marquee byte indexing panics on UTF-8 | 🟡 Medium | sidebar/mod.rs, tree.rs | ~10 lines |
| 10 | No active panel indicator on Mixer/Playback | 🟡 Medium | sound_control, playback | ~10 lines |
| 11 | Help view columns clip at normal widths | 🟡 Medium | help_view.rs | ~20 lines |
| 12 | Progress bar capsule dead zone | 🟡 Medium | progress_bar_view.rs | ~5 lines |
| 13 | Slider value formatting by string match | 🟡 Medium | sound_control_view.rs | ~10 lines |
| 14 | Image cache double-processing on track change | 🟡 Medium | image_cache.rs | ~10 lines |
| 15 | No status bar / footer | 🟢 Low | New widget | ~60 lines |
| 16 | No loading / empty states | 🟢 Low | Multiple widgets | ~30 lines |
| 17 | No panel resize keybinding | 🟢 Low | State + view | ~40 lines |
| 18 | Overlay dim effect inconsistent | 🟢 Low | view.rs | ~15 lines |
| 19 | Confirm dialog no visual boundary | 🟢 Low | confirm_view.rs | ~10 lines |
| 20 | Help view no scroll support | 🟢 Low | help_view.rs | ~30 lines |

---

## Proposed Fix Order

### Sprint 1: Layout Foundation (Fixes rendering at all terminal sizes)

1. **Create `theme.rs`** — centralized color palette (Issue 4, 5)
2. **Fix root layout constraints** — `Min` + `Fill` instead of `Percentage` (Issue 1, 3)
3. **Add minimum terminal size guard** (Issue 2)
4. **Fix notification popup y-position** (Issue 8)
5. **Fix marquee UTF-8 safety** (Issue 9)

### Sprint 2: Focus Mode + Widget Polish

1. **Implement Focus/Zen Mode** (Issue 21) ← User priority
2. **Fix branding art overflow** (Issue 6)
3. **Make track info layout adaptive** (Issue 7)
4. **Add active panel indicators everywhere** (Issue 10)
5. **Fix progress bar capsule dead zone** (Issue 12)
6. **Improve slider value formatting** (Issue 13)

### Sprint 3: UX Enhancement

1. **Add status bar** (Issue 15)
2. **Add loading/empty states** (Issue 16)
3. **Improve overlay dimming** (Issue 18)
4. **Improve confirm dialog visibility** (Issue 19)
5. **Fix help view layout and add scroll** (Issue 11, 20)

### Sprint 4: Advanced

1. **Panel resize keybinding** (Issue 17)
2. **Optimize image cache flow** (Issue 14)
3. **Config file for UI options** (sidebar width, theme, scrolloff, default layout mode)

---

## Design System Proposal

Create a `theme.rs` at `melotui-tui/src/theme.rs`:

```rust
use ratatui::style::Color;

pub struct Theme {
    // Semantic UI colors
    pub border_active: Color,     // Panels with focus
    pub border_inactive: Color,   // Panels without focus
    pub bg_panel: Color,          // Panel backgrounds
    pub bg_selected: Color,       // Selected item background
    pub bg_overlay: Color,        // Overlay/popup background

    // Text hierarchy
    pub fg_primary: Color,        // Main text
    pub fg_secondary: Color,      // Supporting text
    pub fg_muted: Color,          // Disabled/hint text

    // Semantic accents
    pub accent_playing: Color,    // Currently playing track
    pub accent_ancestor: Color,   // Parent of playing track
    pub accent_info: Color,       // Info notifications
    pub accent_warning: Color,    // Warning notifications
    pub accent_error: Color,      // Error notifications

    // Category-specific
    pub icon_folder: Color,
    pub icon_playlist: Color,
    pub icon_category: Color,

    // Mixer slider colors
    pub slider_volume: Color,
    pub slider_speed: Color,
    pub slider_bass: Color,
    pub slider_treble: Color,
    pub slider_reverb: Color,
    pub slider_echo: Color,
}

impl Theme {
    pub fn catppuccin_mocha() -> Self { /* ... */ }
    pub fn tokyo_night() -> Self { /* ... */ }
    pub fn dracula() -> Self { /* ... */ }
    pub fn minimal() -> Self { /* ... */ }
}
```

Every widget would receive `&Theme` and use it instead of inline constants. This enables:

- User-selectable themes via config
- Consistent visual language across all widgets
- Easy creation of new themes
