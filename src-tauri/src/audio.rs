use anyhow::{anyhow, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use serde::Serialize;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

use libpulse_binding::callbacks::ListResult;
use libpulse_binding::context::{Context, FlagSet as ContextFlagSet, State as ContextState};
use libpulse_binding::def::PortAvailable;
use libpulse_binding::mainloop::standard::{IterateResult, Mainloop};
use libpulse_binding::proplist::{properties, Proplist};

pub const TARGET_RATE: u32 = 16_000;

#[derive(Debug, Clone, Serialize)]
pub struct DeviceInfo {
    pub name: String,
    pub description: String,
}

struct Controller {
    stop_tx: mpsc::Sender<()>,
    result_rx: mpsc::Receiver<Vec<f32>>,
}

static CURRENT: Mutex<Option<Controller>> = Mutex::new(None);
static DEVICE_NAME: Mutex<Option<String>> = Mutex::new(None);

pub fn set_device(name: Option<String>) {
    *DEVICE_NAME.lock().unwrap() = name;
}

pub fn list_devices() -> Vec<DeviceInfo> {
    match list_pulse_sources() {
        Ok(list) => list,
        Err(e) => {
            eprintln!("[bvoice] PulseAudio source enumeration failed: {:?}", e);
            Vec::new()
        }
    }
}

fn list_pulse_sources() -> Result<Vec<DeviceInfo>> {
    let mut proplist = Proplist::new().ok_or_else(|| anyhow!("PA proplist init failed"))?;
    proplist
        .set_str(properties::APPLICATION_NAME, "BVoice")
        .map_err(|_| anyhow!("PA set application.name failed"))?;

    let mut mainloop = Mainloop::new().ok_or_else(|| anyhow!("PA mainloop init failed"))?;
    let ctx = Rc::new(RefCell::new(
        Context::new_with_proplist(&mainloop, "BVoice", &proplist)
            .ok_or_else(|| anyhow!("PA context init failed"))?,
    ));

    ctx.borrow_mut()
        .connect(None, ContextFlagSet::NOFLAGS, None)
        .map_err(|e| anyhow!("PA connect failed: {:?}", e))?;

    loop {
        match mainloop.iterate(true) {
            IterateResult::Quit(_) | IterateResult::Err(_) => {
                return Err(anyhow!("PA mainloop iterate failed during connect"));
            }
            IterateResult::Success(_) => {}
        }
        match ctx.borrow().get_state() {
            ContextState::Ready => break,
            ContextState::Failed | ContextState::Terminated => {
                return Err(anyhow!("PA context failed before ready"));
            }
            _ => {}
        }
    }

    let sources: Rc<RefCell<Vec<DeviceInfo>>> = Rc::new(RefCell::new(Vec::new()));
    let done = Rc::new(RefCell::new(false));

    let sources_cb = Rc::clone(&sources);
    let done_cb = Rc::clone(&done);

    let introspect = ctx.borrow().introspect();
    let _op = introspect.get_source_info_list(move |result| match result {
        ListResult::Item(info) => {
            if info.monitor_of_sink.is_some() {
                return;
            }
            if let Some(port) = &info.active_port {
                if port.available == PortAvailable::No {
                    return;
                }
            }
            let name = info.name.as_ref().map(|s| s.to_string()).unwrap_or_default();
            if name.is_empty() {
                return;
            }
            let description = info
                .description
                .as_ref()
                .map(|s| s.to_string())
                .unwrap_or_else(|| name.clone());
            sources_cb
                .borrow_mut()
                .push(DeviceInfo { name, description });
        }
        ListResult::End | ListResult::Error => {
            *done_cb.borrow_mut() = true;
        }
    });

    while !*done.borrow() {
        match mainloop.iterate(true) {
            IterateResult::Quit(_) | IterateResult::Err(_) => break,
            IterateResult::Success(_) => {}
        }
    }

    ctx.borrow_mut().disconnect();

    let mut out: Vec<DeviceInfo> = sources.borrow().clone();
    out.sort_by(|a, b| a.description.cmp(&b.description));
    Ok(out)
}

pub fn start() -> Result<()> {
    let (init_tx, init_rx) = mpsc::channel::<Result<(), String>>();
    let (stop_tx, stop_rx) = mpsc::channel::<()>();
    let (result_tx, result_rx) = mpsc::channel::<Vec<f32>>();

    thread::spawn(move || match build_stream() {
        Err(e) => {
            let _ = init_tx.send(Err(e.to_string()));
        }
        Ok((stream, buffer, source_rate, channels)) => {
            let _ = init_tx.send(Ok(()));
            let _ = stop_rx.recv();
            drop(stream);
            let samples = std::mem::take(&mut *buffer.lock().unwrap());
            let mono = downmix(samples, channels);
            let resampled = resample(&mono, source_rate, TARGET_RATE);
            let _ = result_tx.send(resampled);
        }
    });

    match init_rx.recv() {
        Ok(Ok(())) => {
            *CURRENT.lock().unwrap() = Some(Controller { stop_tx, result_rx });
            Ok(())
        }
        Ok(Err(e)) => Err(anyhow!(e)),
        Err(e) => Err(anyhow!("audio thread died before init: {}", e)),
    }
}

pub fn stop() -> Option<Vec<f32>> {
    let ctrl = CURRENT.lock().unwrap().take()?;
    let _ = ctrl.stop_tx.send(());
    ctrl.result_rx.recv().ok()
}

type StreamBundle = (cpal::Stream, Arc<Mutex<Vec<f32>>>, u32, u16);

fn build_stream() -> Result<StreamBundle> {
    let selected = DEVICE_NAME.lock().unwrap().clone();
    match &selected {
        Some(name) => std::env::set_var("PULSE_SOURCE", name),
        None => std::env::remove_var("PULSE_SOURCE"),
    }

    let host = cpal::default_host();
    let device = if selected.is_some() {
        host.input_devices()?
            .find(|d| d.name().ok().as_deref() == Some("pulse"))
            .or_else(|| host.default_input_device())
            .ok_or_else(|| anyhow!("no pulse or default input device"))?
    } else {
        host.default_input_device()
            .ok_or_else(|| anyhow!("no default input device"))?
    };
    let config = device.default_input_config()?;
    let source_rate = config.sample_rate().0;
    let channels = config.channels();
    let sample_format = config.sample_format();
    let stream_config: cpal::StreamConfig = config.into();

    let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
    let err_fn = |e| eprintln!("[bvoice] cpal stream error: {}", e);

    let stream = match sample_format {
        cpal::SampleFormat::F32 => {
            let buf = Arc::clone(&buffer);
            device.build_input_stream(
                &stream_config,
                move |data: &[f32], _: &_| buf.lock().unwrap().extend_from_slice(data),
                err_fn,
                None,
            )?
        }
        cpal::SampleFormat::I16 => {
            let buf = Arc::clone(&buffer);
            device.build_input_stream(
                &stream_config,
                move |data: &[i16], _: &_| {
                    let mut b = buf.lock().unwrap();
                    b.extend(data.iter().map(|&x| x as f32 / i16::MAX as f32));
                },
                err_fn,
                None,
            )?
        }
        cpal::SampleFormat::U16 => {
            let buf = Arc::clone(&buffer);
            device.build_input_stream(
                &stream_config,
                move |data: &[u16], _: &_| {
                    let mut b = buf.lock().unwrap();
                    b.extend(data.iter().map(|&x| (x as f32 - 32768.0) / 32768.0));
                },
                err_fn,
                None,
            )?
        }
        f => return Err(anyhow!("unsupported sample format {:?}", f)),
    };

    stream.play()?;
    Ok((stream, buffer, source_rate, channels))
}

fn downmix(samples: Vec<f32>, channels: u16) -> Vec<f32> {
    if channels <= 1 {
        return samples;
    }
    let c = channels as usize;
    samples
        .chunks(c)
        .map(|ch| ch.iter().sum::<f32>() / c as f32)
        .collect()
}

fn resample(input: &[f32], from_hz: u32, to_hz: u32) -> Vec<f32> {
    if from_hz == to_hz {
        return input.to_vec();
    }
    use rubato::{FftFixedIn, Resampler};

    let chunk_size = 1024;
    let mut resampler = match FftFixedIn::<f32>::new(
        from_hz as usize,
        to_hz as usize,
        chunk_size,
        2,
        1,
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("[bvoice] resampler init failed, falling back: {:?}", e);
            return linear_resample(input, from_hz, to_hz);
        }
    };

    let mut out = Vec::with_capacity(input.len() * to_hz as usize / from_hz as usize + 64);
    let mut i = 0;
    while i + chunk_size <= input.len() {
        let chunk = vec![input[i..i + chunk_size].to_vec()];
        match resampler.process(&chunk, None) {
            Ok(result) => out.extend_from_slice(&result[0]),
            Err(e) => {
                eprintln!("[bvoice] resampler process failed: {:?}", e);
                return linear_resample(input, from_hz, to_hz);
            }
        }
        i += chunk_size;
    }
    if i < input.len() {
        let tail = vec![input[i..].to_vec()];
        if let Ok(result) = resampler.process_partial(Some(&tail), None) {
            out.extend_from_slice(&result[0]);
        }
    }
    out
}

fn linear_resample(input: &[f32], from_hz: u32, to_hz: u32) -> Vec<f32> {
    let ratio = from_hz as f64 / to_hz as f64;
    let out_len = (input.len() as f64 / ratio) as usize;
    let mut out = Vec::with_capacity(out_len);
    for i in 0..out_len {
        let src = i as f64 * ratio;
        let idx = src.floor() as usize;
        let frac = (src - idx as f64) as f32;
        let s0 = input.get(idx).copied().unwrap_or(0.0);
        let s1 = input.get(idx + 1).copied().unwrap_or(s0);
        out.push(s0 + (s1 - s0) * frac);
    }
    out
}
