use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

static ARM_MS: AtomicU64 = AtomicU64::new(1000);
static TRIGGER: Mutex<rdev::Key> = Mutex::new(rdev::Key::AltGr);
static CAPTURING: AtomicBool = AtomicBool::new(false);
static STATE: Mutex<State> = Mutex::new(State::Idle);
static SEQ: AtomicU64 = AtomicU64::new(0);

pub fn reset() {
    *STATE.lock().unwrap() = State::Idle;
    SEQ.fetch_add(1, Ordering::Relaxed);
}

pub fn set_arm_threshold_ms(ms: u64) {
    ARM_MS.store(ms, Ordering::Relaxed);
}

pub fn set_trigger(key: rdev::Key) {
    *TRIGGER.lock().unwrap() = key;
}

pub fn start_capture() {
    CAPTURING.store(true, Ordering::SeqCst);
}

pub fn key_to_str(key: rdev::Key) -> String {
    format!("{:?}", key)
}

pub fn key_from_str(s: &str) -> Option<rdev::Key> {
    use rdev::Key::*;
    Some(match s {
        "Alt" => Alt,
        "AltGr" => AltGr,
        "ControlLeft" => ControlLeft,
        "ControlRight" => ControlRight,
        "ShiftLeft" => ShiftLeft,
        "ShiftRight" => ShiftRight,
        "MetaLeft" => MetaLeft,
        "MetaRight" => MetaRight,
        "CapsLock" => CapsLock,
        "Tab" => Tab,
        "Space" => Space,
        "Escape" => Escape,
        "Return" => Return,
        "F1" => F1,
        "F2" => F2,
        "F3" => F3,
        "F4" => F4,
        "F5" => F5,
        "F6" => F6,
        "F7" => F7,
        "F8" => F8,
        "F9" => F9,
        "F10" => F10,
        "F11" => F11,
        "F12" => F12,
        _ => return None,
    })
}

#[derive(Debug, Clone, Copy)]
enum State {
    Idle,
    Pending(u64),
    Armed,
}

#[derive(Debug, Clone)]
pub enum HotkeyEvent {
    Cancelled,
    Armed,
    Released,
    Captured(String),
}

pub fn start_listener<F>(on_event: F)
where
    F: Fn(HotkeyEvent) + Send + Sync + 'static,
{
    thread::spawn(move || {
        let on_event = Arc::new(on_event);

        let callback = move |event: rdev::Event| match event.event_type {
            rdev::EventType::KeyPress(k) => {
                if CAPTURING.swap(false, Ordering::SeqCst) {
                    on_event(HotkeyEvent::Captured(key_to_str(k)));
                    return;
                }
                let trigger = *TRIGGER.lock().unwrap();
                if k != trigger {
                    return;
                }
                let mut s = STATE.lock().unwrap();
                if matches!(*s, State::Idle) {
                    let my_seq = SEQ.fetch_add(1, Ordering::Relaxed).wrapping_add(1);
                    *s = State::Pending(my_seq);
                    let ev_for_timer = Arc::clone(&on_event);
                    let threshold = Duration::from_millis(ARM_MS.load(Ordering::Relaxed));
                    thread::spawn(move || {
                        thread::sleep(threshold);
                        let mut s = STATE.lock().unwrap();
                        if let State::Pending(id) = *s {
                            if id == my_seq {
                                *s = State::Armed;
                                drop(s);
                                ev_for_timer(HotkeyEvent::Armed);
                            }
                        }
                    });
                }
            }
            rdev::EventType::KeyRelease(k) => {
                let trigger = *TRIGGER.lock().unwrap();
                if k != trigger {
                    return;
                }
                let mut s = STATE.lock().unwrap();
                let out = match *s {
                    State::Pending(_) => Some(HotkeyEvent::Cancelled),
                    State::Armed => Some(HotkeyEvent::Released),
                    State::Idle => None,
                };
                *s = State::Idle;
                drop(s);
                if let Some(e) = out {
                    on_event(e);
                }
            }
            _ => {}
        };

        if let Err(e) = rdev::listen(callback) {
            eprintln!("[bvoice] rdev listen error: {:?}", e);
        }
    });
}
