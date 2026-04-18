use anyhow::{anyhow, Result};
use std::io::{Read, Write};
use std::path::PathBuf;

const BASE_URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/";

pub fn filename_for(model: &str) -> String {
    format!("ggml-{}.bin", model)
}

pub fn model_path(model: &str) -> PathBuf {
    let mut p = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
    p.push("bvoice");
    p.push("models");
    p.push(filename_for(model));
    p
}

pub fn ensure_model(model: &str) -> Result<PathBuf> {
    ensure_model_with_progress(model, |_, _| {})
}

pub fn ensure_model_with_progress<F>(model: &str, on_progress: F) -> Result<PathBuf>
where
    F: Fn(u64, u64),
{
    let path = model_path(model);
    if path.exists() {
        return Ok(path);
    }
    std::fs::create_dir_all(path.parent().unwrap())?;
    let url = format!("{}{}", BASE_URL, filename_for(model));
    println!("[bvoice] downloading model: {}", url);
    download(&url, &path, on_progress)?;
    println!("[bvoice] model saved: {}", path.display());
    Ok(path)
}

fn download<F>(url: &str, dest: &std::path::Path, on_progress: F) -> Result<()>
where
    F: Fn(u64, u64),
{
    let mut resp = reqwest::blocking::get(url)?;
    if !resp.status().is_success() {
        return Err(anyhow!("download failed: HTTP {}", resp.status()));
    }
    let total = resp.content_length().unwrap_or(0);
    let tmp = dest.with_extension("part");
    let mut file = std::fs::File::create(&tmp)?;
    let mut buf = [0u8; 64 * 1024];
    let mut downloaded: u64 = 0;
    let mut last_emit_pct: i64 = -1;
    loop {
        let n = resp.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        downloaded += n as u64;
        if let Some(pct) = (downloaded * 100).checked_div(total).map(|p| p as i64) {
            if pct != last_emit_pct {
                on_progress(downloaded, total);
                last_emit_pct = pct;
            }
        }
    }
    on_progress(downloaded, total);
    file.sync_all()?;
    drop(file);
    std::fs::rename(&tmp, dest)?;
    Ok(())
}
