use anyhow::{anyhow, Result};
use std::sync::OnceLock;
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
    let default_owned = trim_transparent_square(default_ref);
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

fn trim_transparent_square(img: &Image<'_>) -> Image<'static> {
    let w = img.width() as usize;
    let h = img.height() as usize;
    let rgba = img.rgba();
    let alpha_at = |x: usize, y: usize| rgba[(y * w + x) * 4 + 3];

    let mut min_x = w;
    let mut max_x = 0usize;
    let mut min_y = h;
    let mut max_y = 0usize;
    for y in 0..h {
        for x in 0..w {
            if alpha_at(x, y) > 8 {
                if x < min_x { min_x = x; }
                if x > max_x { max_x = x; }
                if y < min_y { min_y = y; }
                if y > max_y { max_y = y; }
            }
        }
    }
    if min_x > max_x || min_y > max_y {
        return Image::new_owned(rgba.to_vec(), img.width(), img.height());
    }
    let cw = max_x - min_x + 1;
    let ch = max_y - min_y + 1;
    let side = cw.max(ch);
    let pad_x = (side - cw) / 2;
    let pad_y = (side - ch) / 2;

    let mut out = vec![0u8; side * side * 4];
    for y in 0..ch {
        let src_row = (min_y + y) * w + min_x;
        let dst_row = (pad_y + y) * side + pad_x;
        let src_start = src_row * 4;
        let dst_start = dst_row * 4;
        out[dst_start..dst_start + cw * 4]
            .copy_from_slice(&rgba[src_start..src_start + cw * 4]);
    }
    Image::new_owned(out, side as u32, side as u32)
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
