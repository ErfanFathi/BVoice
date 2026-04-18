use anyhow::{anyhow, Result};
use std::path::Path;
use std::sync::RwLock;
use whisper_rs::{FullParams, SamplingStrategy, WhisperContext, WhisperContextParameters};

static CTX: RwLock<Option<WhisperContext>> = RwLock::new(None);

pub fn init(model_path: &Path) -> Result<()> {
    let path_str = model_path
        .to_str()
        .ok_or_else(|| anyhow!("non-utf8 model path"))?;
    let ctx = WhisperContext::new_with_params(path_str, WhisperContextParameters::default())?;
    *CTX.write().unwrap() = Some(ctx);
    Ok(())
}

pub fn is_ready() -> bool {
    CTX.read().unwrap().is_some()
}

pub fn transcribe(samples: &[f32], beam_size: u32) -> Result<String> {
    let guard = CTX.read().unwrap();
    let ctx = guard.as_ref().ok_or_else(|| anyhow!("model not loaded yet"))?;
    let mut state = ctx.create_state()?;
    let sampling = if beam_size >= 2 {
        SamplingStrategy::BeamSearch {
            beam_size: beam_size as i32,
            patience: -1.0,
        }
    } else {
        SamplingStrategy::Greedy { best_of: 1 }
    };
    let mut params = FullParams::new(sampling);
    params.set_language(Some("en"));
    params.set_translate(false);
    params.set_print_special(false);
    params.set_print_progress(false);
    params.set_print_realtime(false);
    params.set_print_timestamps(false);

    state.full(params, samples)?;

    let n = state.full_n_segments();
    let mut out = String::new();
    for i in 0..n {
        if let Some(seg) = state.get_segment(i) {
            out.push_str(&seg.to_str_lossy()?);
        }
    }
    let trimmed = out.trim();
    if is_nonverbal(trimmed) {
        return Ok(String::new());
    }
    Ok(trimmed.to_string())
}

fn is_nonverbal(s: &str) -> bool {
    let t = s.trim().trim_matches(|c: char| c == '.' || c == ' ');
    let lower = t.to_ascii_lowercase();
    t.is_empty()
        || (t.starts_with('[') && t.ends_with(']'))
        || (t.starts_with('(') && t.ends_with(')'))
        || (t.starts_with('*') && t.ends_with('*'))
        || matches!(
            lower.as_str(),
            "blank_audio" | "silence" | "music" | "inaudible" | "no speech"
        )
}

