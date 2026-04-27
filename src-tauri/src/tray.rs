use anyhow::{anyhow, Result};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    App, AppHandle, Manager,
};

const TRAY_ID: &str = "bvoice-tray";

static APP: OnceLock<AppHandle> = OnceLock::new();
static IDLE_ICON: OnceLock<Image<'static>> = OnceLock::new();
static RECORDING_ICON: OnceLock<Image<'static>> = OnceLock::new();
static TRANSCRIBING_ICON: OnceLock<Image<'static>> = OnceLock::new();
static STATE_AT: Mutex<Option<(State, Instant)>> = Mutex::new(None);

#[derive(Debug, Clone, Copy)]
pub enum State {
    Idle,
    Recording,
    Transcribing,
}

pub fn init(app: &App) -> Result<()> {
    let settings_i = MenuItem::with_id(app, "settings", "Settings", true, None::<&str>)?;
    let quit_i = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&settings_i, &quit_i])?;

    let default_ref = app
        .default_window_icon()
        .ok_or_else(|| anyhow!("no default window icon"))?;
    let default_owned =
        Image::new_owned(default_ref.rgba().to_vec(), default_ref.width(), default_ref.height());
    RECORDING_ICON
        .set(tint(&default_owned, [230, 60, 60]))
        .ok();
    TRANSCRIBING_ICON
        .set(tint(&default_owned, [240, 180, 40]))
        .ok();
    IDLE_ICON.set(default_owned.clone()).ok();

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(default_owned)
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
    let Some(app) = APP.get() else { return };
    let Some(tray) = app.tray_by_id(TRAY_ID) else { return };
    let icon = match state {
        State::Idle => IDLE_ICON.get(),
        State::Recording => RECORDING_ICON.get(),
        State::Transcribing => TRANSCRIBING_ICON.get(),
    };
    if let Some(img) = icon {
        let _ = tray.set_icon(Some(img.clone()));
    }
}

pub fn state_with_age() -> Option<(State, Duration)> {
    STATE_AT
        .lock()
        .unwrap()
        .map(|(s, t)| (s, t.elapsed()))
}

fn tint(src: &Image<'_>, color: [u8; 3]) -> Image<'static> {
    let rgba = src.rgba();
    let mut out = Vec::with_capacity(rgba.len());
    for chunk in rgba.chunks(4) {
        out.push(color[0]);
        out.push(color[1]);
        out.push(color[2]);
        out.push(chunk[3]);
    }
    Image::new_owned(out, src.width(), src.height())
}
