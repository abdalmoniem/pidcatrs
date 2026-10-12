<div align="center">
   <img width="200" src="assets/icon.png" alt="pidcatrs icon"/>
   <h1>📃 pidcatrs</h1>
   <p>A colorized Android logcat viewer with an interactive TUI, themes, and advanced filtering</p>

[![GPLv3 License](https://img.shields.io/badge/License-GPL%20v3-yellow.svg)](LICENSE.md)
[![Python](https://img.shields.io/badge/Rust-1.92+-yellow.svg)](https://rust-lang.org)

<!-- [![WinGet Package Version](https://img.shields.io/winget/v/AbdElMoniemElHifnawy.pidcatrs)](https://winstall.app/apps/AbdElMoniemElHifnawy.pidcatrs) -->

[![Views](https://views.whatilearened.today/views/github/abdalmoniem/pidcatrs.svg)](https://github.com/abdalmoniem/pidcatrs)
[![GitHub Release](https://img.shields.io/github/v/release/abdalmoniem/pidcatrs)](https://github.com/abdalmoniem/pidcatrs/releases/latest)
[![GitHub Downloads (all assets, all releases)](https://img.shields.io/github/downloads/abdalmoniem/pidcatrs/total?logo=github&logoSize=auto&label=GitHub%20Downloads)](https://github.com/abdalmoniem/pidcatrs/releases/latest)

</div>

# 🎯 Overview

A fork of [PidCat](https://github.com/JakeWharton/pidcat) re-written entirely in [Rust](https://rust-lang.org).

[PidCat](https://github.com/JakeWharton/pidcat) was originally created by Jake Wharton for the Android Open Source Project. **pidcatrs** extends that idea with a full-screen terminal UI, documented TOML configuration and color themes, multiple adb log formats (including multiline `long` output), timestamps, and the same package/tag filtering workflow you expect from classic pidcat—plus plain streaming mode when stdout is not a TTY or when you pass `--plain`.

pidcatrs filters logcat output by application package name, colorizes logs for readability, and uses one **Logcat-style filter query** everywhere: the `-Q` / `--query` flag, the TUI filter bar, and (when no query is set) legacy `-t` / `-r` / `-l` / package flags compiled into the same syntax. Matching terms can **highlight** corresponding text in the log table while you filter.

---

# 📸 Screenshots

<div align="center">

<table>
  <tr>
    <td align="center" width="33%"><img src="assets/screenshot_1.png" alt="Screenshot 1" width="100%"/></td>
    <td align="center" width="33%"><img src="assets/screenshot_2.png" alt="Screenshot 2" width="100%"/></td>
    <td align="center" width="33%"><img src="assets/screenshot_3.png" alt="Screenshot 3" width="100%"/></td>
  </tr>

  <tr>
    <td align="center" width="33%"><img src="assets/screenshot_4.png" alt="Screenshot 4" width="100%"/></td>
    <td align="center" width="33%"><img src="assets/screenshot_5.png" alt="Screenshot 5" width="100%"/></td>
    <td align="center" width="33%"><img src="assets/screenshot_6.png" alt="Screenshot 6" width="100%"/></td>
  </tr>

  <tr>
    <td align="center" width="33%"><img src="assets/screenshot_7.png" alt="Screenshot 7" width="100%"/></td>
    <td align="center" width="33%"><img src="assets/screenshot_8.png" alt="Screenshot 8" width="100%"/></td>
    <td align="center" width="33%"><img src="assets/screenshot_9.png" alt="Screenshot 9" width="100%"/></td>
  </tr>
</table>

</div>

---

# ✨ Features

- ## Modes

  - **Interactive TUI** — Default when stdout is a terminal: scroll, pause, filter, pick devices, open files, export, clipboard copy.
  - **Plain stream mode** — `--plain`, or automatic when stdout is piped or redirected (same filters and colors as classic pidcat-style output).

- ## Core filtering

  - **Unified filter engine** — Plain mode and the TUI share the same parser and evaluator (`tag:`, `pid:`, `uid:`, `package:`, `level:`, `message`, `age:`). Process start/death banners follow the same rules where applicable.
  - **Logcat-style query** — `-Q` / `--query` for startup filtering; press **`/`** in the TUI to edit the same expression live. Syntax: whitespace = **AND**, `|` = **OR**, `-` = **NOT**, parentheses for grouping, `"quoted"` message literals, `key:value` equality, `key~regex` on supported fields.
  - **Legacy CLI → query** — When `--query` is omitted, `-t`, `-r`, `-l`, and resolved package names are compiled into an equivalent query for the filter bar and plain output. **`--query` cannot be combined** with `-t`, `-r`, `-l`, positional packages, or `--current` (use `-i` / `-I` for tag exclusions either way).
  - **Package filtering** — One or more package names; process suffixes (`com.app:remote`, `com.app:`) supported.
  - **Tag filtering** — Substring matching by default; regex via `tag~pattern` or legacy regex tag patterns on `-t`.
  - **Ignore tags** — `-i` / config `ignore-tag`; optional **ignore known system tags** (`-I`); merged as implicit `-tag:…` clauses (not shown in the filter bar).
  - **Log level** — Minimum level `-l` or `level:info` (comma-separated names take the lowest specified level).
  - **Message regex** — `-r` or `message~pattern` in a query.
  - **Age window** — `age:30s`, `age:5m`, `age:1h`, `age:2d` keep entries newer than the duration (evaluation uses local wall clock).
  - **Live filter (TUI)** — `-L` / `--live-filter` / config `live-filter` (default `true`): re-apply the filter bar on each edit; `--live-filter=false` applies only on **Enter** (invalid text while typing keeps the last good filter and shows an error in the status bar).
  - **Token highlights** — Non-negated query terms (except `level:` / `age:`) highlight matching PID, UID, package, tag, and message text in the log view, including matches across wrapped message lines.
  - **Current app** — `--current` resolves foreground package(s) via adb.
  - **Process tracking** — Start/death notifications for filtered processes.

- ## TUI filter bar

  - **Multiline input** — Soft-wraps while focused; collapses to one line with an ellipsis when unfocused.
  - **Syntax coloring** — Filter key names and `:` / `~` operators use the theme accent color.
  - **Key autocomplete** — **Ctrl+Space** (or typing a key prefix) opens a popup at the cursor with `key:` / `key~` rows; **Tab** or **Enter** inserts the selection; **↑** / **↓** move the list.
  - **Editing shortcuts** — **Ctrl+L** clear; **Ctrl+←** / **Ctrl+→** word motion; **Ctrl+W** / **Ctrl+Del** delete previous/next word.
  - **Export / copy** — **Ctrl+s** export and filtered views respect the effective query (user text plus implicit ignore tags).

- ## Formatting and adb

  - **Log formats** — `-f` / `--log-format`: `brief`, `long`, `process`, `raw`, `tag`, `thread`, `threadtime`, `time` (short aliases supported).
  - **Timestamps** — `-T` / `--timestamps` and `-Z` / `--timestamp-format` (chrono strftime).
  - **Columns** — Show PID (`-P`), UID (`-U`), package (`-p`); widths `-x` (pid/uid), `-m` (package), `-n` (tag); always show tags (`-S`).
  - **Custom adb path** — `-A` / `--adb`.
  - **Device selection** — `-d`, `-e`, `-s SERIAL` (TUI can switch devices with `d`).

- ## Configuration and themes

  - **Config file** — `~/.config/pidcatrs/config.toml` (or platform equivalent); JSON Schema in `schemas/config.schema.json`.
  - **218 bundled color themes** — Installed under `~/.config/pidcatrs/themes/`; default theme `gruber-darker`; custom `.toml` themes supported.
  - **CLI helpers** — `--print-config`, `--print-theme`, `--list-themes`, `--config`, `--theme`.
  - **Shell completions** — `--completions bash|zsh|fish|powershell|elvish`.

- ## Output

  - **Save to file** — `-o` / `--output`.
  - **Colors** — Theme-driven TUI and plain colors; `-N` / `--no-color`; optional GC highlighting `-g` / `--gc-color`.
  - **Cross-platform color** — VT100/ANSI on Windows, Linux, and macOS.

---

# 📥 Installation

- ### Installer binaries

  [<img alt="Get it on GitHub" height="80" src="assets/badge_github.png"/>](https://github.com/abdalmoniem/pidcatrs/releases/latest)
  <!-- [<img alt="Get from WinGet" height="80" src="assets/badge_winget.png"/>](https://winstall.app/apps/AbdElMoniemElHifnawy.pidcatrs) -->

- ## From source

  - ### Prerequisites

    - [Android SDK platform-tools](https://developer.android.com/tools/releases/platform-tools#downloads) with `adb` on `PATH`
    - [Rust](https://rust-lang.org/learn/get-started) (see `rust-toolchain.toml`)
    - [just](https://github.com/casey/just) (recommended) or `cargo` + `cargo xtask`
    - [Inno Setup](https://jrsoftware.org/isdl.php) — Windows installer only

  - ### Quick build

    ```bash
    git clone https://github.com/abdalmoniem/pidcatrs.git
    cd pidcatrs

    # Debug binary (fmt, clippy, schema/themes generation, build)
    just build

    # Release binary
    just build-release

    # Install release binary to PATH (non-Windows: cargo install style via xtask)
    just install
    ```

  - ### Run without installing

    ```bash
    just run 'com.example.app'
    just run-release '--plain com.example.app'
    ```

---

# 🚀 Usage

- ## TUI vs plain

  - With a connected device and a TTY stdout, **pidcatrs starts the TUI** automatically.
  - Use **`--plain`** for traditional scrolling line output (scripts, CI, or copying from a terminal without full-screen UI).
  - Redirecting stdout (e.g. `pidcatrs … | tee log.txt`) also selects plain mode.

- ## Basic usage

```bash
# TUI: filter logs by package name
pidcatrs com.example.myapp

# Multiple packages
pidcatrs com.example.app1 com.example.app2

# Plain stream
pidcatrs --plain com.example.myapp

# All packages
pidcatrs -a

# Foreground app only
pidcatrs --current
```

- ## Advanced filtering

```bash
# Tags (repeat or comma-separated)
pidcatrs com.example.app -t MyTag -t AnotherTag
pidcatrs com.example.app -t MyTag,AnotherTag

# Substring tag match
pidcatrs com.example.app -t Timeout

# Level and ignored tags
pidcatrs com.example.app -l D -i Chatty -i Verbose

# Message regex
pidcatrs com.example.app -r "Exception|FATAL"

# Regex tag patterns
pidcatrs com.example.app -t "^Network.*"

# Single Logcat-style query (replaces -t / -r / -l / packages for filtering)
pidcatrs -Q 'tag:OkHttp | tag:Retrofit level:debug Exception'

# Plain mode with the same query
pidcatrs --plain -Q 'message~NullPointer age:10m' com.example.app

# TUI: apply filter only when you press Enter
pidcatrs -L false com.example.app
```

- ## Command line options

  Positional **`PACKAGE`** — Application package name(s); may be repeated. On the command line, packages **replace** the `packages` list in config.

  | Option | Description |
  | --- | --- |
  | `-h`, `--help` | Show help and exit |
  | `-v`, `--version` | Show version and exit |
  | `--completions SHELL` | Write shell completions to stdout |
  | `-A`, `--adb ADB_PATH` | Path to `adb` if not on `PATH` |

  **Device**

  | Option | Description |
  | --- | --- |
  | `-d`, `--device` | First physical device |
  | `-e`, `--emulator` | First emulator |
  | `-s`, `--serial SERIAL` | Specific device serial |

  **Filtering**

  | Option | Description |
  | --- | --- |
  | `-a`, `--all` | All packages (no package filter) |
  | `-k`, `--keep` | Do not clear logcat before tailing |
  | `-c`, `--current` | Filter to current foreground app(s) |
  | `-I`, `--ignore-system-tags` | Ignore built-in system tag list (combine with `-i`) |
  | `-t`, `--tag TAG` | Include tag(s); repeat or comma-separated |
  | `-i`, `--ignore-tag TAG` | Exclude tag(s); repeat or comma-separated |
  | `-l`, `--log-level LEVEL` | Minimum level: `V` `D` `I` `W` `E` `F` (case insensitive) |
  | `-r`, `--regex REGEX` | Filter log message body |
  | `-Q`, `--query QUERY` | Logcat-style filter query (cannot mix with `-t`, `-r`, `-l`, packages, or `--current`) |
  | `-L`, `--live-filter BOOL` | TUI: apply filter while typing (default `true`; `false` = apply on Enter only) |

  **Formatting**

  | Option | Description |
  | --- | --- |
  | `-T`, `--timestamps` | Show timestamp column |
  | `-Z`, `--timestamp-format FMT` | chrono strftime format (default `%I:%M:%S%.3f%P`) |
  | `-f`, `--log-format FORMAT` | adb format: brief, long, process, raw, tag, thread, threadtime, time |
  | `-P`, `--show-pid` | Show PID column |
  | `-U`, `--show-uid` | Show UID column |
  | `-p`, `--show-package` | Show package / process name column |
  | `-S`, `--always-show-tags` | Always show tag column |
  | `-x`, `--puid-width WIDTH` | PID/UID column width (default 5) |
  | `-m`, `--package-width WIDTH` | Package column width (default 20) |
  | `-n`, `--tag-width WIDTH` | Tag column width (default 20) |

  **Colors and output**

  | Option | Description |
  | --- | --- |
  | `-g`, `--gc-color` | Highlight GC-related messages |
  | `-N`, `--no-color` | Disable message colors |
  | `-o`, `--output FILE` | Write plain output to file |
  | `--plain` | Force plain stream mode (no TUI) |

  **Config**

  | Option | Description |
  | --- | --- |
  | `--config PATH` | Config file (default `~/.config/pidcatrs/config.toml`) |
  | `--theme NAME_OR_PATH` | Bundled theme name or path to `.toml` (default `gruber-darker`) |
  | `--list-themes` | List bundled and custom themes |
  | `--print-config` | Print effective config as documented TOML |
  | `--print-theme` | Print active theme as documented TOML |

  CLI flags override config values when both are set. See `schemas/config.schema.json` for every config key.

---

# 🖥️ TUI user guide

Press **`?`** anytime to open the **command palette** (searchable list of shortcuts). **`Esc`** closes dialogs or leaves filter editing.

- ## General

  | Keys | Action |
  | --- | --- |
  | `q`, `Ctrl+c` | Quit |
  | `?` | Open command palette |
  | `Esc` | Close dialog / stop editing filter |

- ## Log view

  | Keys | Action |
  | --- | --- |
  | `j` / `k`, `↑` / `↓`, mouse wheel | Scroll |
  | `PgUp` / `PgDn` | Scroll by page |
  | `g` / `Home` | Jump to top |
  | `G` / `End` | Jump to bottom (resume live tail) |
  | `v` | Enter select mode |

- ## Select mode

  | Keys | Action |
  | --- | --- |
  | `j` / `k`, `↑` / `↓` | Move selection |
  | `PgUp` / `PgDn` | Move by 10 entries |
  | `y`, `Enter` | Open copy menu for selected entry |
  | `v`, `Esc` | Exit select mode |

- ## Log capture

  | Keys | Action |
  | --- | --- |
  | `p`, `Space` | Pause / resume incoming logs |
  | `l` | Clear buffer and restart live logcat |
  | `d` | Open device picker |
  | `o` | Open log file (file explorer) |
  | `Ctrl+s` | Export entries matching the current filter |

- ## Filter bar

  | Keys | Action |
  | --- | --- |
  | `/` | Focus filter input |
  | `Enter` | Apply filter and leave the bar (live updates while typing unless `--live-filter=false`) |
  | `Esc` | Close autocomplete, or exit the bar |
  | `←` / `→`, `Home` / `End` | Move cursor in filter text |
  | `Backspace` / `Delete` | Edit filter text (bar expands while focused) |
  | `Ctrl+Space` | Open or refresh key autocomplete at the cursor |
  | `Tab` / `↑` / `↓` / `Enter` | Choose a completion from the popup |
  | `Ctrl+L` | Clear the filter bar |
  | `Ctrl+←` / `Ctrl+→` | Move cursor by word |
  | `Ctrl+W` / `Ctrl+Del` | Delete previous / next word |

  **Filter syntax** (same as `-Q` / `--query`; space = AND):

  | Syntax | Meaning |
  | --- | --- |
  | `package:com.example.app` | Filter by package (multiple terms = AND) |
  | `tag:MyTag` | Tag substring (or `tag~regex`) |
  | `level:debug` | Minimum level (`verbose`, `debug`, `info`, …) |
  | `pid:1234` | PID match (`pid~` for regex) |
  | `uid:10001` | UID match |
  | `message~Error.*` | Regex on message body |
  | `word` / `"phrase"` | Message substring (implicit `message:`) |
  | `(a \| b)` | OR (use parentheses to group) |
  | `-tag:Noise` | NOT (leading `-` on a term) |
  | `age:5m` | Entries newer than duration (`s` / `m` / `h` / `d`) |

- ## Device picker

  | Keys | Action |
  | --- | --- |
  | `Enter` | Select highlighted device |
  | `Ctrl+r` | Refresh device list |
  | `j` / `k`, `↑` / `↓` | Move selection |

- ## Copy menu

  | Keys | Action |
  | --- | --- |
  | `m` / `t` / `p` / `u` / `e` | Copy message, tag, pid, uid, or full entry |
  | `Enter` | Copy highlighted option |
  | `j` / `k`, `↑` / `↓` | Move selection |

- ## Export format

  | Keys | Action |
  | --- | --- |
  | `p` / `a` | Export as rendered output vs raw adb lines |
  | `Enter` | Confirm format |
  | `j` / `k`, `↑` / `↓` | Move selection |

- ## File explorer

  | Keys | Action |
  | --- | --- |
  | Type a path | List matching directory entries |
  | `↑` / `↓`, `PgUp` / `PgDn` | Move selection |
  | `Tab` | Complete path with selected entry |
  | `Enter` | Open file or save export (confirm twice to overwrite) |

---

# 📚 Examples

<details>

<summary>Example 1: Basic package filtering</summary>

```bash
pidcatrs com.example.myapp
```

Opens the TUI (or plain stream if stdout is not a TTY) for `com.example.myapp`.

</details>

<details>

<summary>Example 2: Multiple tags with custom column widths</summary>

```bash
pidcatrs com.example.myapp -t Network -t Database -m 25 -n 30
```

Filters tags containing `Network` or `Database`; package column 25 chars, tag column 30 chars.

</details>

<details>

<summary>Example 3: Debug level and above</summary>

```bash
pidcatrs com.example.myapp -l D
```

Shows Debug, Info, Warning, Error, and Fatal (hides Verbose).

</details>

<details>

<summary>Example 4: Save to file without colors</summary>

```bash
pidcatrs com.example.myapp -o logs.txt -N --plain
```

Writes plain, uncolored lines to `logs.txt`.

</details>

<details>

<summary>Example 5: Current app with specific tags</summary>

```bash
pidcatrs --current -t MainActivity -t ServiceManager -S
```

Monitors the foreground app and always shows tag names for matching tags.

</details>

<details>

<summary>Example 6: Long log format with timestamps</summary>

```bash
pidcatrs --plain -f long -T com.example.myapp
```

Uses adb `long` format (multiline messages) with a timestamp column in plain mode.

</details>

<details>

<summary>Example 7: Config and theme</summary>

```bash
pidcatrs --print-config > ~/.config/pidcatrs/config.toml
pidcatrs --list-themes
pidcatrs --theme catppuccin-mocha com.example.app
```

Print a documented config template, list themes, and run with a bundled theme.

</details>

<details>

<summary>Example 8: Shell completions</summary>

```bash
pidcatrs --completions zsh > "${fpath[1]}/_pidcatrs"
```

Generate zsh completions (adjust path for your setup).

</details>

<details>

<summary>Example 9: Logcat-style query on the CLI</summary>

```bash
pidcatrs -Q '(tag:OkHttp | tag:Retrofit) level:info -tag:Spam message~timeout'
```

One query replaces separate `-t`, `-l`, and `-r` flags. Tag ignore lists from `-i` still apply invisibly.

</details>

<details>

<summary>Example 10: Live filter and config</summary>

```bash
# config.toml (Filtering options)
live-filter = false

pidcatrs com.example.app
```

In the TUI, edit the filter bar with **/**; changes apply only when you press **Enter**. Default is live filtering on every keystroke (`live-filter = true` or omit the key).

</details>

---

# 🔨 Building from source

- ## Prerequisites

  - Android SDK platform-tools (`adb` on `PATH`)
  - Rust toolchain from `rust-toolchain.toml`
  - `just` (see [Justfile](Justfile)) or invoke `cargo xtask` directly
  - Inno Setup — only for `just build-installer` on Windows

- ## Build steps

  ```bash
  just build              # debug
  just build-release      # release
  just test               # workspace tests
  just generate-schema    # refresh schemas/config.schema.json
  just generate-themes    # refresh bundled theme sources
  ```

  - ### Windows installer:

    ```bash
    just build-installer
    # Output: build/setup/Output/pidcatrs_v<version>_<datetime>.exe
    ```

  - ### Binaries:

    `target/debug/pidcatrs` or `target/release/pidcatrs` (`.exe` on Windows).

---

# ⚙️ Configuration

- ## Files

  - **Config**: `~/.config/pidcatrs/config.toml` (Linux/macOS) or `%APPDATA%\pidcatrs\config.toml` (Windows).
  - **Themes**: `themes/` next to the config file; bundled themes are copied on first run.
  - **Schemas**: [`schemas/config.schema.json`](schemas/config.schema.json), [`schemas/theme.schema.json`](schemas/theme.schema.json) for editor validation.

  On first run, if no config exists, pidcatrs can write a default file from built-in defaults. Use `--print-config` to dump the effective settings as a documented template.

  **`live-filter`** (boolean, default `true`) lives under **Filtering options** in the generated template. **`--query`** is CLI-only today (not a config key); put a default query in a shell alias or wrapper if you need one.

- ## Query vs legacy flags

  | Approach | When to use |
  | --- | --- |
  | `-Q '…'` | Single expressive filter; required if you already use query syntax in Logcat Studio or adb search. |
  | `-t` / `-r` / `-l` / packages | Classic pidcat-style flags; compiled to a query for display and filtering when `-Q` is unset. |
  | `-i` / `-I` | Always allowed; exclusions are not shown in the filter bar but still apply. |

- ## Tag filtering behavior

  By default, tag filters use **substring matching**:

  ```bash
  -t Timeout
  ```

  Matches tags containing `Timeout` (`TimeoutJob`, `NetworkTimeout`, …). Use regex metacharacters for exact or pattern matches:

  ```bash
  -t "^TimeoutJob$"
  -t "Timeout.*Job"
  ```

- ## Column widths

  ```bash
  pidcatrs com.example.app -m 25 -n 30 -x 6
  ```

  Package width `-m`, tag width `-n`, PID/UID width `-x`. Longer names are truncated.

- ## Colors

  Plain and TUI colors come from the active **theme** TOML (log levels, tags, packages, UI chrome). Tags and packages receive stable colors from an LRU palette; common Android tags have predefined colors in many themes.

---

# 🤝 Contributing

Contributions are welcome! Here's how you can help:

- ## Fork the repository
- ## Create a feature branch
  ```bash
  git checkout -b feature/amazing-feature
  ```
- ## Commit your changes
  ```bash
  git commit -m 'Add some amazing feature'
  ```
- ## Push to the branch
  ```bash
  git push origin feature/amazing-feature
  ```
- ## Open a Pull Request

- ## Development guidelines

  - Run `just build` or `just lint` before submitting
  - Add comments for non-obvious logic
  - Test on your target OS (Windows, Linux, macOS)
  - Update README and schemas when adding flags or config keys

---

# 📄 License

This project is licensed under the GNU General Public License 3.0 - see the [LICENSE](LICENSE.md) file for details.

---

# 🙏 Credits

- ## Original author

  - **[Jake Wharton](https://github.com/JakeWharton)** — Original [PidCat](https://github.com/JakeWharton/pidcat) creator

- ## Fork maintainer

  - **AbdElMoniem ElHifnawy** — Rust rewrite, TUI, themes, and cross-platform enhancements
  - GitHub: [@abdalmoniem](https://github.com/abdalmoniem)
  - Website: [abdalmoniem-alhifnawy.is-a.dev](https://abdalmoniem-alhifnawy.is-a.dev)

- ## Contributors

  Thanks to everyone who has helped improve pidcatrs!

---

<div align="center">

**Made with ❤️ for Android developers**

If you find pidcatrs useful, please ⭐ star the repository!

</div>
