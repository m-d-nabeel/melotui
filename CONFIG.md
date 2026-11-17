# Configuring melotui

`melotui --paths` prints where everything is kept. The settings are in `config.toml` in the config folder (`~/.config/melotui/` on Linux); a file that is missing, or a setting left out, means the default. A setting melotui does not know is reported when it starts.

Every setting, with its default:

```toml
[library]
music_directories = []            # folders to scan; empty means the system's music folder
# download_directory = "..."      # default: Downloads in the first music directory; has to be inside one of them
# download_cookies_from_browser = "firefox"   # let yt-dlp use this browser's YouTube cookies (see the README)
# db_path = "..."                 # default: library.db in the data folder
scan_on_startup = true            # pick up music added while melotui was closed
watch = true                      # scan again when the folders change; turn off on a slow network share
max_cover_resolution = 512        # album art is scaled down to this many pixels
picker_viewport_size = 20         # rows shown in the playlist picker

[audio]
gapless = true                    # one track straight into the next
# output_device = "USB Audio"     # a device's name, or part of it; default follows the system
sample_rate = 48000               # the output's rate in Hz
buffer_size = 8192                # the output's buffer in frames; larger is safer under load
seek_step = 5                     # seconds the arrow keys seek by
volume_step = 0.05                # how much + and - change the volume (1.0 is all of it)
speed_step = 0.1                  # how much [ and ] change the speed
fft_size = 4096                   # samples the visualizer analyses at a time
default_speed = 1.0               # 0.5 to 2.0
default_bass = 1.0                # 0.1 to 5.0
default_treble = 1.0              # 0.1 to 5.0
default_reverb = 0.0              # 0.0 to 1.0
default_echo = 0.0                # 0.0 to 0.9
default_pan = 0.0                 # -1.0 (left) to 1.0 (right)
default_lowpass = 20000.0         # Hz, 200 to 20000
default_highpass = 20.0           # Hz, 20 to 5000

[lyrics]
auto_fetch = true                 # fetch lyrics from lrclib.net for a song that has none
offset_ms = 0                     # show lyrics this much later; negative for earlier

[ui]
theme = "default"                 # a built-in theme (see below), or a file in the themes folder
theme_background = true           # paint the theme's background; false keeps the terminal's (for transparency)
sidebar_width = 28                # columns, 28 to 80
scrolloff = 8                     # rows kept in sight above and below the selection
notification_duration_secs = 2
art_max_height_percent = 40       # how much of the sidebar's height the track info's cover may take
fps_limit = 60
restore_session = true            # start where the last session ended
lyrics_shaping = true             # draw Hindi, Urdu, Arabic and the like as images where the terminal can
lyrics_zoom = 2.0                 # lyrics size in the panel, 1 to 4
lyrics_zoom_focus = 3.0           # and in the full lyrics view
queue_add_debounce_ms = 500       # how soon the same track can be queued again
playlist_picker_max_visible = 10
```

## Themes

`:theme <name>` switches theme while melotui runs (Tab completes the names) and is remembered next time, until you change `theme` in `config.toml`. `:theme` on its own says which is in use.

Built in, each with its project's own colours:

| Dark | Light |
| --- | --- |
| `catppuccin_mocha` (the default), `catppuccin_macchiato`, `catppuccin_frappe`, `dracula`, `everforest_dark`, `gruvbox_dark`, `kanagawa`, `nord`, `one_dark`, `rose_pine`, `solarized_dark`, `tokyo_night` | `catppuccin_latte`, `gruvbox_light`, `solarized_light` |

The default keeps your terminal's background and text colour; the others paint their own, unless `theme_background = false`.

### Your own

A theme is a `.toml` file in the `themes` folder inside the config folder (`melotui --paths` shows where), named for how you choose it: `themes/mine.toml` is `theme = "mine"`. Colours are hex (`"#bd93f9"`), a terminal colour's name (`"red"`, `"light blue"`), a number from 0 to 255, or `"reset"` for the terminal's own. Editing a theme while you use it, `:theme` with its name again reads it afresh.

The easy way is a palette of sixteen colours. Everything melotui draws is worked out from them, so these alone make a whole theme:

```toml
background = "#1a1b26"   # "reset" keeps the terminal's
surface = "#16161e"      # pickers, dialogs, the command bar
surface_high = "#292e42" # what is selected
border = "#3b4261"       # borders of what does not have the keys, slider tracks
foreground = "#c0caf5"
subtext = "#a9b1d6"
muted = "#565f89"
accent = "#7aa2f7"       # what has the keys, and selections
red = "#f7768e"
orange = "#ff9e64"
yellow = "#e0af68"
green = "#9ece6a"        # what is playing
cyan = "#7dcfff"
blue = "#7aa2f7"
purple = "#9d7cd8"
pink = "#bb9af7"
```

A file named for a built-in theme changes that theme: `themes/nord.toml` with only `accent = "#ebcb8b"` is Nord with a yellow accent. A file with another name starts from the default, or from the built-in theme it names as `base = "dracula"`.

Any single colour can be given too, after the palette:

| For | Names |
| --- | --- |
| Borders | `border_active`, `border_inactive`, `border_panel` (the visualizer, album art and what is playing) |
| Backgrounds | `bg_main`, `bg_selected`, `bg_overlay`, `bg_input`, `bg_dim` (behind a popup) |
| Text | `fg_primary`, `fg_secondary`, `fg_muted`, `fg_selected` (on the accent) |
| The tree | `fg_tree_root`, `fg_tree_folder`, `fg_tree_subfolder`, `fg_tree_file` |
| Accents | `accent_primary`, `accent_playing`, `accent_ancestor`, `accent_info`, `accent_warning`, `accent_error` |
| Playback status | `status_playing`, `status_paused`, `status_stopped` |
| Icons | `icon_folder`, `icon_playlist`, `icon_category` |
| Sliders | `slider_volume`, `slider_speed`, `slider_bass`, `slider_treble`, `slider_reverb`, `slider_echo`, `slider_track`, `slider_pan`, `slider_lowpass`, `slider_highpass` |
| DSP studio | `dsp_section_border` (the equaliser), `dsp_selected_bg`, `eq_bar_fill`, `eq_bar_empty`, `eq_bar_selected`, `eq_zero_line` |
| Help | `help_key`, `help_header`, `help_text`, `help_subtext`, `help_overlay_bg`, `help_border` |
| Track info | `badge_lossless`, `badge_lossy`, `badge_detail_bg` |
| Visualizer | `visualizer_low`, `visualizer_mid`, `visualizer_high` (from the quietest or lowest to the loudest or highest) |

A name melotui does not know, or a value that is not a colour, is said when the theme is chosen, and the rest of the file is still used.

## Keys

Keys are changed in `keymaps.toml` in the config folder, a section for each place keys are read (`global`, `library`, `queue`, `mixer`, `dsp`, `lyrics`, `download`, `playlist`, `preset`, `trackinfo`, `confirm`, `command`, `help`), each line a key and what it does:

```toml
[global]
"<C-p>" = "PlayPause"      # Ctrl+P; <S-...> is Shift, <M-...> Alt
"<F6>" = "ToggleLyrics"

[library]
"x" = "CollapseTree"
```

`?` in melotui lists every key as it is bound.
