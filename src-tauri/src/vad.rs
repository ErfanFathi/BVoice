use voice_activity_detector::VoiceActivityDetector;

const CHUNK_SIZE: usize = 512; // 32 ms at 16 kHz
const SAMPLE_RATE: i64 = 16_000;
pub const DEFAULT_PAD_CHUNKS: usize = 4; // ~128 ms of context preserved

pub fn trim_silence_with(samples: Vec<f32>, threshold: f32, pad_chunks: usize) -> Vec<f32> {
    if samples.len() < CHUNK_SIZE {
        return samples;
    }

    let mut vad = match VoiceActivityDetector::builder()
        .sample_rate(SAMPLE_RATE)
        .chunk_size(CHUNK_SIZE)
        .build()
    {
        Ok(v) => v,
        Err(e) => {
            eprintln!("[bvoice] VAD init failed, skipping trim: {:?}", e);
            return samples;
        }
    };

    let mut speech: Vec<bool> = Vec::new();
    for chunk in samples.chunks(CHUNK_SIZE) {
        if chunk.len() < CHUNK_SIZE {
            break;
        }
        let p = vad.predict(chunk.to_vec());
        speech.push(p > threshold);
    }

    let first = speech.iter().position(|&s| s);
    let last = speech.iter().rposition(|&s| s);

    match (first, last) {
        (Some(f), Some(l)) => {
            let start = f.saturating_sub(pad_chunks) * CHUNK_SIZE;
            let end = ((l + 1 + pad_chunks) * CHUNK_SIZE).min(samples.len());
            samples[start..end].to_vec()
        }
        _ => Vec::new(),
    }
}
