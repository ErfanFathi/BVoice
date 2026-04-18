<p align="center">
  <img src="src-tauri/icons/icon.png" width="128" alt="BVoice" />
</p>

<h1 align="center">BVoice</h1>

<p align="center">
  Local push-to-talk speech-to-text desktop app.<br/>
  Hold a key, speak, release — the transcription is typed at your cursor.<br/>
  Runs 100% offline using whisper.cpp.
</p>

## Features

- Push-to-talk trigger (default: Right-Alt), configurable via rebind
- Local transcription with whisper.cpp (`tiny.en` / `base.en` / `small.en`)
- Silero VAD trims silence before transcription
- FFT-based resampling (rubato) for high-quality 48 kHz → 16 kHz downsampling
- Beam search (configurable size, default 5) or greedy decoding
- Live-applied settings for threshold, input device, and model swap (no restart)
- Tray icon reflects state (idle / recording / transcribing) with the branded icon
- Single-instance enforcement; optional autostart on login
- Types output directly at the cursor — never touches your clipboard

## Platform support

- **Linux / X11** — primary target, tested on Ubuntu GNOME
- **Wayland** — not supported (global hotkeys and synthesized paste require compositor-specific portals)
- **macOS / Windows** — not yet ported

## Build from source

### Prerequisites

- Rust (stable) via [rustup](https://rustup.rs)
- Node.js 20+ and npm
- Tauri CLI: `cargo install tauri-cli --version '^2.0' --locked`
- Linux system packages (Ubuntu/Debian):

  ```
  sudo apt install libwebkit2gtk-4.1-dev libsoup-3.0-dev \
    libayatana-appindicator3-dev libclang-dev pkg-config libssl-dev
  ```

### Run

```
npm install
npm run tauri dev        # development
npm run tauri build      # release bundles (AppImage + .deb)
```

On first run the chosen whisper model (~75–466 MB) is downloaded to `~/.local/share/bvoice/models/`.

## Configuration

Settings are stored in `~/.config/bvoice/config.toml`:

| Key                | Type          | Default   | Description                                       |
|--------------------|---------------|-----------|---------------------------------------------------|
| `model`            | string        | `base.en` | Whisper model (`tiny.en`, `base.en`, `small.en`)  |
| `arm_threshold_ms` | u64           | `1000`    | Hold duration before recording arms               |
| `input_device`     | string\|null  | `null`    | Input device name; null = system default          |
| `hotkey`           | string        | `AltGr`   | Trigger key (rebindable from the settings window) |
| `beam_size`        | u32           | `5`       | Beam search size; `1` = greedy                    |

All fields are editable from the Settings window and persist on Save.

## Architecture

```
hotkey (rdev, X11 XRecord)  ─▶ state machine (arm-on-1s)
                                   │
                             armed ▼
                             audio::start   (cpal, dedicated thread)
                                   │
                          released ▼
                             audio::stop    (mono + rubato 16 kHz)
                                   │
                                   ▼
                             vad::trim_silence  (Silero VAD)
                                   │
                                   ▼
                             transcribe::transcribe  (whisper-rs, beam search)
                                   │
                                   ▼
                             inject::paste  (enigo — types at cursor)
```

## License

MIT — see [LICENSE](LICENSE).
