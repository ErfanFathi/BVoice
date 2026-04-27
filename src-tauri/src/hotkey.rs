use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;

static CTRL_HELD: AtomicBool = AtomicBool::new(false);
static META_HELD: AtomicBool = AtomicBool::new(false);
static STATE: Mutex<State> = Mutex::new(State::Idle);

pub fn reset() {
    *STATE.lock().unwrap() = State::Idle;
    CTRL_HELD.store(false, Ordering::Relaxed);
    META_HELD.store(false, Ordering::Relaxed);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    Idle,
    Armed,
}

#[derive(Debug, Clone)]
pub enum HotkeyEvent {
    Armed,
    Released,
}

fn is_ctrl(k: rdev::Key) -> bool {
    matches!(k, rdev::Key::ControlLeft | rdev::Key::ControlRight)
}

fn is_meta(k: rdev::Key) -> bool {
    matches!(k, rdev::Key::MetaLeft | rdev::Key::MetaRight)
}

pub fn start_listener<F>(on_event: F)
where
    F: Fn(HotkeyEvent) + Send + Sync + 'static,
{
    thread::spawn(move || {
        let callback = move |event: rdev::Event| match event.event_type {
            rdev::EventType::KeyPress(k) => {
                let touched = if is_ctrl(k) {
                    CTRL_HELD.store(true, Ordering::Relaxed);
                    true
                } else if is_meta(k) {
                    META_HELD.store(true, Ordering::Relaxed);
                    true
                } else {
                    false
                };
                if !touched {
                    return;
                }
                if CTRL_HELD.load(Ordering::Relaxed) && META_HELD.load(Ordering::Relaxed) {
                    let mut s = STATE.lock().unwrap();
                    if *s == State::Idle {
                        *s = State::Armed;
                        drop(s);
                        on_event(HotkeyEvent::Armed);
                    }
                }
            }
            rdev::EventType::KeyRelease(k) => {
                let touched = if is_ctrl(k) {
                    CTRL_HELD.store(false, Ordering::Relaxed);
                    true
                } else if is_meta(k) {
                    META_HELD.store(false, Ordering::Relaxed);
                    true
                } else {
                    false
                };
                if !touched {
                    return;
                }
                let mut s = STATE.lock().unwrap();
                if *s == State::Armed {
                    *s = State::Idle;
                    drop(s);
                    on_event(HotkeyEvent::Released);
                }
            }
            _ => {}
        };

        if let Err(e) = rdev::listen(callback) {
            eprintln!("[bvoice] rdev listen error: {:?}", e);
        }
    });
}
