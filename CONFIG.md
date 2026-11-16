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
theme = "default"                 # or the name of a file in the themes folder
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

The default colours are Catppuccin Mocha. A theme is a file of colours in the `themes` folder inside the config folder: `[ui] theme = "dracula"` reads `themes/dracula.toml`. It is read when melotui starts.

A theme file sets as many of these as it wants, each to a colour such as `"#bd93f9"`; the rest keep their defaults:

| For | Names |
| --- | --- |
| Borders | `border_active`, `border_inactive` |
| Backgrounds | `bg_selected`, `bg_overlay`, `bg_input` |
| Text | `fg_primary`, `fg_secondary`, `fg_muted` |
| The tree | `fg_tree_root`, `fg_tree_folder`, `fg_tree_subfolder`, `fg_tree_file` |
| Accents | `accent_primary`, `accent_playing`, `accent_ancestor`, `accent_info`, `accent_warning`, `accent_error` |
| Playback status | `status_playing`, `status_paused`, `status_stopped` |
| Icons | `icon_folder`, `icon_playlist`, `icon_category` |
| Mixer sliders | `slider_volume`, `slider_speed`, `slider_bass`, `slider_treble`, `slider_reverb`, `slider_echo`, `slider_track`, `slider_pan`, `slider_lowpass`, `slider_highpass` |
| DSP studio | `dsp_section_border`, `dsp_selected_bg`, `eq_bar_fill`, `eq_bar_empty`, `eq_bar_selected`, `eq_zero_line` |
| Help | `help_key`, `help_header`, `help_text`, `help_subtext`, `help_overlay_bg`, `help_border` |
| Track info | `badge_lossless`, `badge_lossy`, `badge_detail_bg` |

For example, `themes/dracula.toml`:

```toml
border_active = "#bd93f9"
border_inactive = "#6272a4"
bg_selected = "#44475a"
fg_primary = "#f8f8f2"
accent_primary = "#bd93f9"
accent_playing = "#50fa7b"
```

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
