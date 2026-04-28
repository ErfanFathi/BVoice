use anyhow::{anyhow, Result};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    App, AppHandle, Emitter, Manager,
};

const TRAY_ID: &str = "bvoice-tray";

static APP: OnceLock<AppHandle> = OnceLock::new();
static STATE_AT: Mutex<Option<(State, Instant)>> = Mutex::new(None);

#[derive(Debug, Clone, Copy)]
pub enum State {
    Idle,
    Recording,
    Transcribing,
}

impl State {
    fn as_str(self) -> &'static str {
        match self {
            State::Idle => "idle",
            State::Recording => "recording",
            State::Transcribing => "transcribing",
        }
    }
}

pub fn init(app: &App) -> Result<()> {
    let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings_i, &quit_i])?;

    let icon = app
        .default_window_icon()
        .ok_or_else(|| anyhow!("no default window icon"))?
        .clone();

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(true)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => app.exit(0),
            "settings" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            _ => {}
        })
        .build(app)?;

    APP.set(app.handle().clone()).ok();
    Ok(())
}

pub fn set_state(state: State) {
    *STATE_AT.lock().unwrap() = Some((state, Instant::now()));
    if let Some(app) = APP.get() {
        let _ = app.emit("bvoice:state", state.as_str());
    }
}

pub fn state_with_age() -> Option<(State, Duration)> {
    STATE_AT
        .lock()
        .unwrap()
        .map(|(s, t)| (s, t.elapsed()))
}
