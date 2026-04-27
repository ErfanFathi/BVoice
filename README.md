<p align="center">
  <img src="src-tauri/icons/icon.png" width="128" alt="BVoice" />
</p>

<h1 align="center">BVoice</h1>

<p align="center">
  Local push-to-talk speech-to-text desktop app.<br/>
  Hold a key, speak, release — the transcription is typed at your cursor.<br/>
  Runs 100% offline using whisper.cpp.
</p>

<p align="center">
  <a href="https://github.com/ErfanFathi/BVoice/releases/latest">
    <img src="https://img.shields.io/github/v/release/ErfanFathi/BVoice?label=latest&color=7552eb" alt="Latest release" />
  </a>
  <img src="https://img.shields.io/badge/platform-linux-informational" alt="Platform" />
  <img src="https://img.shields.io/badge/license-MIT-green" alt="License" />
</p>

---

## Install

Grab the latest build from the [**Releases page**](https://github.com/ErfanFathi/BVoice/releases/latest).

**Debian / Ubuntu** — `.deb`:
```
sudo apt install ./BVoice_0.1.1_amd64.deb
```

**Fedora / RHEL / openSUSE** — `.rpm`:
```
sudo dnf install ./BVoice-0.1.1-1.x86_64.rpm
```

After install you'll find BVoice in your application menu. On first launch the selected whisper model (~75–466 MB) downloads to `~/.local/share/bvoice/models/`.

## Features

- Push-to-talk trigger: hold **Ctrl + Win** (instant arm — no hold delay)
- Local transcription with whisper.cpp (`tiny.en` / `base.en` / `small.en`, full or quantized `q5_1` / `q8_0`)
- Optional Silero VAD silence trim with tunable threshold
- FFT-based resampling (rubato) for high-quality 48 kHz → 16 kHz conversion
- Beam search (configurable size, default 2) or greedy decoding
- Live-applied settings for threshold, input device, and model swap — no restart
- Tray icon reflects state (idle / recording / transcribing) using the branded icon
- Single-instance enforcement; optional autostart on login
- Types output directly at the cursor — never touches your clipboard

## Usage

1. Launch BVoice — a tray icon appears (no window by default).
2. Click the tray icon → Settings to configure model, input device, beam size, VAD, and autostart.
3. Focus any text field (editor, terminal, browser, …).
4. **Hold Ctrl + Win, speak, release.**
5. The transcription is typed at the cursor.

## Platform support

- **Linux / X11** — primary target, tested on Ubuntu GNOME
- **Wayland** — not supported (global hotkeys and synthesized typing require compositor-specific portals)
- **macOS / Windows** — not yet ported

## Configuration

Settings persist at `~/.config/bvoice/config.toml`:

| Key                | Type          | Default   | Description                                                |
|--------------------|---------------|-----------|------------------------------------------------------------|
| `model`            | string        | `base.en` | Whisper model; append `-q5_1` or `-q8_0` for quantized     |
| `input_device`     | string\|null  | `null`    | Input device name; null = system default                   |
| `beam_size`        | u32           | `2`       | Beam search size; `1` = greedy                             |
| `use_vad`          | bool          | `false`   | Trim silence with Silero VAD before transcription          |
| `vad_threshold`    | f32           | `0.5`     | VAD speech probability threshold (0–1); active when on     |

The trigger is hardcoded to **Ctrl + Win** and is not user-configurable.

All fields are editable from the Settings window and persist on Save.

## Build from source

### Prerequisites

- Rust (stable) via [rustup](https://rustup.rs)
- Node.js 20+ and npm
- Tauri CLI: `cargo install tauri-cli --version '^2.0' --locked`
- Linux system packages (Ubuntu/Debian):
  ```
  sudo apt install \
    libwebkit2gtk-4.1-dev libsoup-3.0-dev libayatana-appindicator3-dev \
    libasound2-dev libpulse-dev libxdo-dev libclang-dev libssl-dev libstdc++-12-dev \
    pkg-config build-essential
  ```

### Run / build

```
npm install
npm run tauri dev          # development
npm run tauri build        # release bundles (.deb + .rpm)
```

## Architecture

```
setup (background thread):  model::ensure_model  ─▶  transcribe::init  (whisper-rs context)

hotkey (rdev, X11 XRecord)  ─▶ state machine (Ctrl+Win chord)
                                   │
                             armed ▼
                             audio::start          (cpal capture on dedicated thread,
                                                    PulseAudio source via libpulse-binding)
                                   │
                          released ▼
                             audio::stop           (downmix to mono → rubato 48→16 kHz)
                                   │
                       (if use_vad) ▼
                             vad::trim_silence_with  (Silero VAD, configurable threshold)
                                   │
                                   ▼
                             transcribe::transcribe  (whisper-rs, beam_size≥2 → beam search,
                                                      else greedy; nonverbal segments filtered)
                                   │
                                   ▼
                             inject::paste         (enigo.text() — types at cursor,
                                                    no clipboard)

watchdog thread: forces reset if Recording > 60s or Transcribing > 45s
```

## License

MIT — see [LICENSE](LICENSE).
