# ZFetch

**ZFetch** is a fast, terminal-focused system information fetcher written in Rust. It pairs detailed hardware/OS stats with colorized ASCII logos sourced from Fastfetch, while keeping startup times tiny.

## Quick start

```bash
git clone https://github.com/anshnk/zfetch.git
cd zfetch
cargo build --release
./target/release/ZFetch
```

## Example output

```
                     ..'          ┌──────────────────────────────────────────────┐
                 ,xNMM.           │ ansh@24m3MBA-2.local | System Information     │
               .OMMMMo            ├──────────────────────────────────────────────┤
               lMM"               │  Distro    : Mac OS (26.0.0)                  │
     .;loddo:.  .olloddol;.       │  Distro ID : macos                            │
   cKMMMMMMMMMMNWMMMMMMMMMM0:     │  Kernel    : 25.0.0                           │
 .KMMMMMMMMMMMMMMMMMMMMMMMWd.     │  CPU       : Apple M3 (8 cores) (4.06 GHz)    │
 XMMMMMMMMMMMMMMMMMMMMMMMX.       │  GPU       : Apple M3                         │
;MMMMMMMMMMMMMMMMMMMMMMMM:        │  Memory    : 12.12 GB / 16.00 GB (76%)        │
:MMMMMMMMMMMMMMMMMMMMMMMM:        │  Swap      : 3.35 GB / 4.00 GB (84%)          │
.MMMMMMMMMMMMMMMMMMMMMMMMX.       │  Local IP  : 10.157.127.7                     │
 kMMMMMMMMMMMMMMMMMMMMMMMMWd.     │  Battery   : 27% [AC Connected]               │
 'XMMMMMMMMMMMMMMMMMMMMMMMMMMk    │  Uptime    : 4d 18h 39m                       │
  'XMMMMMMMMMMMMMMMMMMMMMMMMK.    │  Disk (/)  : 430.38 GB / 460.40 GB (93%) - apfs│
    kMMMMMMMMMMMMMMMMMMMMMMd      └──────────────────────────────────────────────┘
     ;KMMMMMMMWXXWMMMMMMMk.
       "cooc*"    "*coo'"
```

## Configuration

ZFetch reads `config.json` from the directory that contains the executable (`target/release/ZFetch` when developing). Every field is optional—defaults enable all modules and auto-select colors based on the terminal theme and logo palette.

### Options

| Key | Type | Default | Description |
| --- | --- | --- | --- |
| `show_distro`, `show_distro_id`, `show_kernel`, `show_cpu`, `show_gpu`, `show_memory`, `show_swap`, `show_local_ip`, `show_battery`, `show_storage`, `show_uptime` | `bool` | `true` | Toggle individual info lines. |
| `show_user_host` | `bool` | `true` | Control the `user@host` title. |
| `logo_color` | string list | distro defaults | Up to nine colors applied to `$1…$9` tokens in the ASCII logo. |
| `color` | string | auto | Default value color for info text. |
| `color_keys` | string | auto/distro | Color for labels (`Distro`, `CPU`, …). |
| `color_title` | string | auto/distro | Color used for the panel title. |
| `box_outline_color` | string | auto (bright blue on dark themes, blue on light) | Border color for the info box. |

### Color formats

Color values are parsed by `terminal::colors::parse_color_spec` and accept multiple syntaxes:

- Named colors: `"green"`, `"bright_magenta"`, `"bold+cyan"`.
- Raw ANSI fragments: `"94"`, `"1;32"`, or combos such as `"bold+94"`.
- 256-color palette: `"256:178"`, `"color256:39"`, or shorthand `"178"`.
- Truecolor: `"#ff8800"`, `"rgb(255,136,0)"`, or `"rgb:255;136;0"`.
- Bare escape sequences: `"38;2;255;136;0"`.

`logo_color` accepts space or comma separated values; only the first nine entries are used.

### Example configs

The [`examples/`](examples) directory showcases every color format:

- [`config.colors.named.json`](examples/config.colors.named.json) – named + bright colors.
- [`config.colors.ansi.json`](examples/config.colors.ansi.json) – raw ANSI sequences (e.g., `94`, `1;32`).
- [`config.colors.256.json`](examples/config.colors.256.json) – 256-color palette indices.
- [`config.colors.truecolor_hex.json`](examples/config.colors.truecolor_hex.json) – `#rrggbb` values.
- [`config.colors.truecolor_rgb.json`](examples/config.colors.truecolor_rgb.json) – `rgb()` and `rgb:` syntax.
- [`config.colors.mixed.json`](examples/config.colors.mixed.json) – mixing bold, palette, truecolor, and default reset values.

Copy any file to `config.json` (or adapt it) to preview a palette instantly.

## Benchmarking runtime
The helper script repeatedly runs the release binary, parses the `Execution time:` line, and plots the results with matplotlib:
**Note:** To benchmark runtime, you must uncomment the debugging lines in `main.rs` that print the `Execution time:` output. This is a required step—without it, the helper script will not be able to parse timing information from ZFetch's output.

```bash
cargo build --release
python scripts/plot_zfetch_times.py --runs 100 --output zfetch_times.png
```

Useful flags:

- `--smooth-window` – moving-average overlay window (default 25, set to 1 to disable).
- `--args` – forward additional CLI arguments directly to `ZFetch` for experimentation.

## Roadmap

- [ ] Unify disk/storage detection across platforms.
- [ ] Extend logo overrides and theme presets.
- [ ] Improve GPU probing on Linux/Windows.
- [x] Make colors configurable for keys/title/border.
- [x] Respect terminal theme automatically.

## Credits

- ASCII logos: [Fastfetch](https://github.com/fastfetch-cli/fastfetch)

## License

MIT

Made with 🦀 Rust
