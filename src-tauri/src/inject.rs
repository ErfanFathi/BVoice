use anyhow::{anyhow, Result};
use enigo::{Enigo, Keyboard, Settings};

pub fn paste(text: &str) -> Result<()> {
    if text.is_empty() {
        return Ok(());
    }
    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| anyhow!("enigo init: {:?}", e))?;
    enigo
        .text(text)
        .map_err(|e| anyhow!("enigo text: {:?}", e))?;
    Ok(())
}
