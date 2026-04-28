mod audio;
mod config;
mod hotkey;
mod inject;
mod model;
mod transcribe;
mod tray;
mod vad;

use hotkey::HotkeyEvent;
use serde::Serialize;
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use tauri::{Emitter, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

static OVERLAY_PENDING_POS: Mutex<Option<(i32, i32, Instant)>> = Mutex::new(None);

#[derive(Serialize, Clone)]
struct DownloadProgress {
    model: String,
    downloaded: u64,
    total: u64,
    pct: u32,
}

#[tauri::command]
fn get_config() -> config::Config {
    config::load()
}

#[tauri::command]
fn set_config(app: tauri::AppHandle, new: config::Config) -> Result<(), String> {
    let old = config::load();
    let model_changed = old.model != new.model;
    audio::set_device(new.input_device.clone());
    config::save(&new).map_err(|e| e.to_string())?;
    if model_changed {
        let new_model = new.model.clone();
        thread::spawn(move || {
            println!("[bvoice] reloading model: {}", new_model);
            let app_for_progress = app.clone();
            let model_for_progress = new_model.clone();
            let progress = move |downloaded: u64, total: u64| {
                let pct = downloaded
                    .checked_mul(100)
                    .and_then(|x| x.checked_div(total))
                    .unwrap_or(0) as u32;
                let _ = app_for_progress.emit(
                    "bvoice:download-progress",
                    DownloadProgress {
                        model: model_for_progress.clone(),
                        downloaded,
                        total,
                        pct,
                    },
                );
            };
            match model::ensure_model_with_progress(&new_model, progress)
                .and_then(|p| transcribe::init(&p))
            {
                Ok(_) => {
                    println!("[bvoice] model ready: {}", new_model);
                    let _ = app.emit("bvoice:model-ready", new_model.clone());
                }
                Err(e) => {
                    eprintln!("[bvoice] model reload failed: {:?}", e);
                    let _ = app.emit("bvoice:model-error", format!("{:?}", e));
                }
            }
        });
    }
    Ok(())
}

#[tauri::command]
fn list_input_devices() -> Vec<audio::DeviceInfo> {
    audio::list_devices()
}

pub fn run() {
    let cfg = config::load();
    audio::set_device(cfg.input_device.clone());

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None,
        ))
        .invoke_handler(tauri::generate_handler![
            get_config,
            set_config,
            list_input_devices
        ])
        .setup(move |app| {
            tray::init(app)?;

            let overlay = WebviewWindowBuilder::new(
                app,
                "overlay",
                WebviewUrl::App("index.html".into()),
            )
            .title("BVoice Overlay")
            .inner_size(96.0, 96.0)
            .decorations(false)
            .transparent(true)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            .visible(true)
            .build()?;

            if let Some((x, y)) = cfg.overlay_position {
                let _ = overlay.set_position(PhysicalPosition::new(x, y));
            } else if let (Ok(Some(monitor)), Ok(size)) =
                (overlay.primary_monitor(), overlay.outer_size())
            {
                let m_pos = monitor.position();
                let m_size = monitor.size();
                let margin: i32 = 32;
                let x = m_pos.x + m_size.width as i32 - size.width as i32 - margin;
                let y = m_pos.y + m_size.height as i32 - size.height as i32 - margin;
                let _ = overlay.set_position(PhysicalPosition::new(x, y));
            }

            thread::spawn(|| loop {
                thread::sleep(Duration::from_millis(300));
                let mut guard = OVERLAY_PENDING_POS.lock().unwrap();
                if let Some((x, y, t)) = *guard {
                    if t.elapsed() >= Duration::from_millis(400) {
                        let mut cfg = config::load();
                        cfg.overlay_position = Some((x, y));
                        let _ = config::save(&cfg);
                        *guard = None;
                    }
                }
            });

            let model_name = cfg.model.clone();
            thread::spawn(move || {
                match model::ensure_model(&model_name).and_then(|p| transcribe::init(&p)) {
                    Ok(_) => println!("[bvoice] model ready: {}", model_name),
                    Err(e) => eprintln!("[bvoice] model init failed: {:?}", e),
                }
            });

            hotkey::start_listener(move |event| match event {
                HotkeyEvent::Armed => {
                    if !transcribe::is_ready() {
                        eprintln!("[bvoice] model still loading — ignoring hold");
                        return;
                    }
                    if let Err(e) = audio::start() {
                        eprintln!("[bvoice] audio start failed: {:?}", e);
                    } else {
                        tray::set_state(tray::State::Recording);
                        println!("[bvoice] armed — recording");
                    }
                }
                HotkeyEvent::Released => {
                    let samples = match audio::stop() {
                        Some(s) => s,
                        None => {
                            eprintln!("[bvoice] released but no recording in progress");
                            tray::set_state(tray::State::Idle);
                            return;
                        }
                    };
                    let secs = samples.len() as f32 / audio::TARGET_RATE as f32;
                    println!("[bvoice] released — {:.2}s captured", secs);

                    if !transcribe::is_ready() {
                        eprintln!("[bvoice] model still loading — skipped");
                        tray::set_state(tray::State::Idle);
                        return;
                    }
                    tray::set_state(tray::State::Transcribing);
                    let cfg = config::load();
                    let beam_size = cfg.beam_size;
                    let use_vad = cfg.use_vad;
                    let vad_threshold = cfg.vad_threshold;
                    thread::spawn(move || {
                        let prepared = if use_vad {
                            vad::trim_silence_with(samples, vad_threshold, vad::DEFAULT_PAD_CHUNKS)
                        } else {
                            samples
                        };
                        if prepared.is_empty() {
                            println!("[bvoice] no speech detected");
                            tray::set_state(tray::State::Idle);
                            return;
                        }
                        let t0 = std::time::Instant::now();
                        match transcribe::transcribe(&prepared, beam_size) {
                            Ok(text) => {
                                println!(
                                    "[bvoice] transcribed ({}ms): {:?}",
                                    t0.elapsed().as_millis(),
                                    text
                                );
                                if let Err(e) = inject::paste(&text) {
                                    eprintln!("[bvoice] paste failed: {:?}", e);
                                }
                            }
                            Err(e) => eprintln!("[bvoice] transcribe error: {:?}", e),
                        }
                        tray::set_state(tray::State::Idle);
                    });
                }
            });
            println!("[bvoice] listener up — hold Ctrl+Win to record");

            thread::spawn(|| loop {
                thread::sleep(std::time::Duration::from_secs(2));
                let Some((state, age)) = tray::state_with_age() else {
                    continue;
                };
                let stuck = match state {
                    tray::State::Recording => age > std::time::Duration::from_secs(60),
                    tray::State::Transcribing => age > std::time::Duration::from_secs(45),
                    tray::State::Idle => false,
                };
                if stuck {
                    eprintln!(
                        "[bvoice] watchdog: {:?} stuck {:.1}s, forcing reset",
                        state,
                        age.as_secs_f32()
                    );
                    let _ = audio::stop();
                    hotkey::reset();
                    tray::set_state(tray::State::Idle);
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| match event {
            WindowEvent::CloseRequested { api, .. } if window.label() == "main" => {
                let _ = window.hide();
                api.prevent_close();
            }
            WindowEvent::Moved(pos) if window.label() == "overlay" => {
                *OVERLAY_PENDING_POS.lock().unwrap() = Some((pos.x, pos.y, Instant::now()));
            }
            _ => {}
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
