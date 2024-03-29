#![windows_subsystem = "windows"]
use druid::{Data, PlatformError};
use rust_rawinput::{Input, KeyState, Receiver};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use winapi::um::winuser::{INPUT_u, SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP};

mod ui;

#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq, Eq, Data)]
struct AutoKey {
    key: KeyInput,
    #[data(same_fn = "PartialEq::eq")]
    delay: Duration,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Data)]
struct KeyInput(#[data(same_fn = "PartialEq::eq")] Input);

impl From<&AutoKey> for INPUT {
    fn from(k: &AutoKey) -> Self {
        let key = match k.key.0 {
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

type TimerEventData = (AutoKey, winapi::um::winnt::HANDLE);

unsafe extern "system" fn timer_event_callback(
    data: winapi::um::winnt::PVOID,
    _: winapi::um::winnt::BOOLEAN,
) {
    use winapi::um::threadpoollegacyapiset::DeleteTimerQueueTimer;
    let mut event_data: Box<TimerEventData> = Box::from_raw(data as _);
    let event_data = event_data.as_mut();
    let auto_key = &event_data.0;

    send_key_event(auto_key);

    DeleteTimerQueueTimer(std::ptr::null_mut(), event_data.1, std::ptr::null_mut());
}

unsafe fn add_timer_event(due_time: u32, data: AutoKey) {
    use winapi::um::threadpoollegacyapiset::CreateTimerQueueTimer;
    let event_data: Box<TimerEventData> = Box::new((data, std::ptr::null_mut()));
    let event_data = Box::leak(event_data);
    CreateTimerQueueTimer(
        &mut event_data.1 as _,
        std::ptr::null_mut(),
        Some(timer_event_callback),
        event_data as *mut TimerEventData as _,
        due_time,
        0,
        0,
    );
}

unsafe fn send_key_event(key: &AutoKey) {
    let mut input: INPUT = key.into();
    input.u.ki_mut().dwFlags = 0;
    SendInput(1, &mut input, std::mem::size_of::<INPUT>() as _);

    let mut input: INPUT = key.into();
    input.u.ki_mut().dwFlags = KEYEVENTF_KEYUP;
    SendInput(1, &mut input, std::mem::size_of::<INPUT>() as _);
}

use rand::prelude::*;
unsafe fn send_key_events(keys: impl IntoIterator<Item = AutoKey>, rng: &mut impl Rng) {
    for key in keys {
        add_timer_event((key.delay.as_millis() + rng.gen_range(0, 20)) as _, key);
    }
}

fn main() -> Result<(), PlatformError> {
    ui::show_ui()
}
