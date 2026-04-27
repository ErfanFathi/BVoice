use anyhow::{anyhow, Result};
use std::process::Command;

// enigo's X11 backend leaves enough time between synthesized events that
// Claude Code's TUI input loop drops some characters (notably spaces).
// `xdotool type --delay 0` batches XTest events through libxdo back-to-back
// and works reliably (verified against the same TUI). Subprocess cost is
// trivial compared to whisper inference latency.
pub fn paste(text: &str) -> Result<()> {
    if text.is_empty() {
        return Ok(());
    }
    let status = Command::new("xdotool")
        .args(["type", "--delay", "0", "--"])
        .arg(text)
        .status()
        .map_err(|e| anyhow!("xdotool spawn failed: {} (is xdotool installed?)", e))?;
    if !status.success() {
        return Err(anyhow!("xdotool exited with {}", status));
    }
    Ok(())
}
