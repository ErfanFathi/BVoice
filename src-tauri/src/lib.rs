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
use std::thread;
use tauri::{Emitter, Manager, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

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
    hotkey::set_arm_threshold_ms(new.arm_threshold_ms);
    audio::set_device(new.input_device.clone());
    if let Some(k) = hotkey::key_from_str(&new.hotkey) {
        hotkey::set_trigger(k);
    }
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
fn list_input_devices() -> Vec<String> {
    audio::list_devices()
}

#[tauri::command]
fn capture_hotkey() {
    hotkey::start_capture();
}

pub fn run() {
    let cfg = config::load();
    let threshold_ms = cfg.arm_threshold_ms;
    hotkey::set_arm_threshold_ms(threshold_ms);
    audio::set_device(cfg.input_device.clone());
    if let Some(k) = hotkey::key_from_str(&cfg.hotkey) {
        hotkey::set_trigger(k);
    }

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
            list_input_devices,
            capture_hotkey
        ])
        .setup(move |app| {
            tray::init(app)?;

            let model_name = cfg.model.clone();
            thread::spawn(move || {
                match model::ensure_model(&model_name).and_then(|p| transcribe::init(&p)) {
                    Ok(_) => println!("[bvoice] model ready: {}", model_name),
                    Err(e) => eprintln!("[bvoice] model init failed: {:?}", e),
                }
            });

            let app_handle = app.handle().clone();
            hotkey::start_listener(move |event| match event {
                HotkeyEvent::Armed => {
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
                    let beam_size = config::load().beam_size;
                    thread::spawn(move || {
                        let trimmed = vad::trim_silence(samples);
                        if trimmed.is_empty() {
                            println!("[bvoice] no speech detected");
                            tray::set_state(tray::State::Idle);
                            return;
                        }
                        let t0 = std::time::Instant::now();
                        match transcribe::transcribe(&trimmed, beam_size) {
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
                HotkeyEvent::Cancelled => println!("[bvoice] cancelled"),
                HotkeyEvent::Captured(key) => {
                    println!("[bvoice] hotkey captured: {}", key);
                    let _ = app_handle.emit("bvoice:hotkey-captured", key);
                }
            });
            println!(
                "[bvoice] listener up — hold {} >{}ms to record",
                cfg.hotkey, threshold_ms
            );

            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
