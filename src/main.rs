use rust_rawinput::{Input, KeyState, Receiver};
use std::time::Duration;
use winapi::um::winuser::{INPUT_u, SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP};
use serde::{Serialize, Deserialize};

mod ui;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
struct AutoKey {
    key: Input,
    delay: Duration,
}

impl From<&AutoKey> for INPUT {
    fn from(k: &AutoKey) -> Self {
        let key = match k.key {
            Input::KeyBoard(i) | Input::Mouse(i) => i,
        };
        let mut u = INPUT_u::default();
        unsafe {
            *u.ki_mut() = KEYBDINPUT {
                wVk: key as _,
                wScan: 0,
                dwFlags: 0,
                time: 0,
                dwExtraInfo: 0,
            };
        }
        INPUT {
            type_: INPUT_KEYBOARD,
            u,
        }
    }
}

type TimerEventData = (AutoKey, KeyState, winapi::um::winnt::HANDLE);

unsafe extern "system" fn timer_event_callback(
    data: winapi::um::winnt::PVOID,
    _: winapi::um::winnt::BOOLEAN,
) {
    use winapi::um::threadpoollegacyapiset::DeleteTimerQueueTimer;
    let mut event_data: Box<TimerEventData> = Box::from_raw(data as _);
    let event_data = event_data.as_mut();
    let auto_key = &event_data.0;
    let key_state = &event_data.1;

    send_key_event(auto_key, *key_state);

    DeleteTimerQueueTimer(std::ptr::null_mut(), event_data.2, std::ptr::null_mut());
}

unsafe fn add_timer_event(due_time: u32, data: AutoKey, key_state: KeyState) {
    use winapi::um::threadpoollegacyapiset::CreateTimerQueueTimer;
    let event_data: Box<TimerEventData> = Box::new((data, key_state, std::ptr::null_mut()));
    let event_data = Box::leak(event_data);
    CreateTimerQueueTimer(
        &mut event_data.2 as _,
        std::ptr::null_mut(),
        Some(timer_event_callback),
        event_data as *mut TimerEventData as _,
        due_time,
        0,
        0,
    );
}

unsafe fn send_key_event(key: &AutoKey, state: KeyState) {
    let mut input: INPUT = key.into();
    input.u.ki_mut().dwFlags = if let KeyState::Down = state {
        0
    } else {
        KEYEVENTF_KEYUP
    };
    SendInput(1, &mut input, std::mem::size_of::<INPUT>() as _);
}

use rand::prelude::*;
unsafe fn send_key_events(
    keys: impl IntoIterator<Item = AutoKey>,
    state: KeyState,
    rng: &mut impl Rng,
) {
    for key in keys {
        add_timer_event(
            (key.delay.as_millis() + rng.gen_range(0, 20)) as _,
            key,
            state,
        );
    }
}

use iced::Application;
fn main() {
    ui::UIState::run(iced::Settings::default());
}
