use crate::AutoKey;
use crate::Receiver;
use crate::{send_key_events, KeyInput};
use crate::{Input, KeyState};
use druid::{
    self, im, text::format::ParseFormatter, widget, AppDelegate, AppLauncher, Color, Command, Data,
    DelegateCtx, Env, Event, EventCtx, Handled, Lens, PlatformError, Target, Widget, WidgetExt,
    WindowConfig, WindowDesc, WindowLevel,
};
use rand::prelude::*;
use serde_json;
use std::fs::OpenOptions;

const KEY_INPUT: druid::Selector<Input> = druid::Selector::new("event.send-key-input");
const DEL_KEY: druid::Selector<AutoKey> = druid::Selector::new("event.del-key");

#[derive(Debug, Clone, Data, Lens)]
struct UIState {
    macro_keys: im::Vector<AutoKey>,
    hotkey: Option<KeyInput>,
    ready_to_press_hotkey: bool,
    sub_window: SubWinState,
    #[data(ignore)]
    rng: SmallRng,
    on_off_state: bool,
}

impl UIState {
    fn new(keys: Vec<AutoKey>, hotkey: Option<KeyInput>) -> Self {
        Self {
            macro_keys: keys.into(),
            hotkey,
            ready_to_press_hotkey: Default::default(),
            sub_window: Default::default(),
            rng: SmallRng::from_entropy(),
            on_off_state: Default::default(),
        }
    }
    fn to_save_data(&self) -> (Vec<&AutoKey>, &Option<KeyInput>) {
        (self.macro_keys.iter().collect(), &self.hotkey)
    }
}

#[derive(Debug, Clone, Data, Lens, Default)]
struct SubWinState {
    key: Option<KeyInput>,
    delay: f64,
    ready_to_press: bool,
}

lazy_static::lazy_static! {
    static ref SAVE_FILE_PATH: std::path::PathBuf = dirs::home_dir().unwrap_or_default().join("flask_macro.config");
}

fn key_event_thread_fn(event_sink: druid::ExtEventSink) {
    let mut receiver = Receiver::new();

    loop {
        match futures::executor::block_on(receiver.get()) {
            Some((key, state)) if state == KeyState::Down => {
                if event_sink
                    .submit_command(KEY_INPUT, key, druid::Target::Auto)
                    .is_err()
                {
                    break;
                }
            }
            Some(_) => {}
            _ => break,
        }
    }
}

pub fn show_ui() -> Result<(), PlatformError> {
    let (keys, hotkey): (Vec<AutoKey>, Option<KeyInput>) = OpenOptions::new()
        .read(true)
        .open(&*SAVE_FILE_PATH)
        .ok()
        .and_then(|file| serde_json::from_reader(file).ok())
        .unwrap_or_default();

    let launcher = AppLauncher::with_window(
        WindowDesc::new(build_ui())
            .title("Flask Macro")
            .window_size(druid::Size::new(400., 400.)),
    );
    let event_sink = launcher.get_external_handle();
    std::thread::spawn(move || key_event_thread_fn(event_sink));

    launcher.delegate(DelKeyDelegater).launch(UIState::new(keys, hotkey))
}

fn build_ui() -> impl Widget<UIState> {
    widget::Flex::column()
        .with_default_spacer()
        .with_child(
            widget::Scroll::new(widget::Either::new(
                |data: &UIState, _| data.macro_keys.is_empty(),
                widget::Label::new("None"),
                widget::List::new(build_hotkey)
                    .with_spacing(20.)
                    .lens(UIState::macro_keys),
            ))
            .border(Color::WHITE, 2.),
        )
        .with_flex_spacer(10.)
        .with_child(
            widget::Flex::row()
                .with_child(
                    widget::Label::dynamic(|data: &UIState, _| {
                        data.hotkey.map(|v| format!("{}", v.0)).unwrap_or_else(|| {
                            if data.ready_to_press_hotkey {
                                "Press Hotkey".to_owned()
                            } else {
                                "None".to_owned()
                            }
                        })
                    })
                    .controller(HotKeySelector),
                )
                .with_default_spacer()
                .with_child(
                    widget::Button::new("Change Hotkey")
                        .on_click(|_, data: &mut bool, _| *data = true)
                        .lens(UIState::ready_to_press_hotkey),
                ),
        )
        .with_default_spacer()
        .with_child(widget::Switch::new().lens(UIState::on_off_state))
        .with_flex_spacer(1.)
        .with_child(
            widget::Flex::row()
                .with_child(widget::Button::new("Add Key").on_click(
                    |ctx, data: &mut UIState, env| {
                        let window_config = WindowConfig::default()
                            .window_size_policy(druid::WindowSizePolicy::User)
                            .resizable(true)
                            .window_size(druid::Size::new(400., 400.))
                            .set_level(WindowLevel::Modal);
                        ctx.new_sub_window(
                            window_config,
                            build_subwindow(),
                            data.clone(),
                            env.clone(),
                        );
                    },
                ))
                .with_default_spacer()
                .with_child(widget::Button::new("Save Config").on_click(
                    |_, data: &mut UIState, _| {
                        if let Ok(file) = OpenOptions::new()
                            .write(true)
                            .truncate(true)
                            .create(true)
                            .open(&*SAVE_FILE_PATH)
                        {
                            serde_json::to_writer(file, &data.to_save_data()).ok();
                        }
                    },
                )),
        )
        .with_default_spacer()
}

fn build_hotkey() -> impl Widget<AutoKey> {
    widget::Flex::row()
        .with_child(widget::Label::dynamic(|data: &AutoKey, _| {
            format!("Key: {}", data.key.0)
        }))
        .with_default_spacer()
        .with_child(widget::Label::dynamic(|data: &AutoKey, _| {
            format!("Delay: {:.2} s", data.delay.as_secs_f64())
        }))
        .with_default_spacer()
        .with_child(
            widget::Button::new("Del").on_click(|ctx, data: &mut AutoKey, _| {
                ctx.submit_command(druid::Command::new(
                    DEL_KEY,
                    data.clone(),
                    druid::Target::Auto,
                ));
            }),
        )
}

fn build_subwindow() -> impl Widget<UIState> {
    widget::Flex::column()
        .with_child(
            widget::Flex::row()
                .with_child(
                    widget::Label::dynamic(|data: &SubWinState, _| {
                        data.key.map(|key| format!("{}", key.0)).unwrap_or_else(|| {
                            if data.ready_to_press {
                                "Press Any Key".to_owned()
                            } else {
                                "None".to_owned()
                            }
                        })
                    })
                    .controller(KeySelector)
                    .lens(UIState::sub_window),
                )
                .with_default_spacer()
                .with_child(
                    widget::Button::new("Select Key")
                        .on_click(|_ctx, data: &mut SubWinState, _| {
                            data.key = None;
                            data.ready_to_press = true;
                        })
                        .lens(UIState::sub_window),
                ),
        )
        .with_default_spacer()
        .with_child(
            widget::ValueTextBox::new(
                widget::TextBox::new(),
                ParseFormatter::<f64>::with_format_fn(|v| format!("{:.2}", v)),
            )
            .update_data_while_editing(true)
            .lens(druid::lens::Then::new(
                UIState::sub_window,
                SubWinState::delay,
            )),
        )
        .with_default_spacer()
        .with_child(
            widget::Flex::row()
                .with_child(
                    widget::Button::new("Add").on_click(|ctx, data: &mut UIState, _| {
                        if let Some(key) = data.sub_window.key {
                            let delay = std::time::Duration::from_secs_f64(data.sub_window.delay);
                            data.macro_keys.push_back(AutoKey { key, delay });
                            data.sub_window = Default::default();
                            ctx.window().close();
                        }
                    }),
                )
                .with_default_spacer()
                .with_child(widget::Button::new("Cancel").on_click(
                    |ctx, data: &mut UIState, _| {
                        data.sub_window = Default::default();
                        ctx.window().close()
                    },
                )),
        )
}

struct KeySelector;

impl<W: Widget<SubWinState>> widget::Controller<SubWinState, W> for KeySelector {
    fn event(
        &mut self,
        _child: &mut W,
        _ctx: &mut EventCtx,
        event: &Event,
        data: &mut SubWinState,
        _env: &Env,
    ) {
        match event {
            Event::Command(cmd) if cmd.is(KEY_INPUT) && data.ready_to_press => {
                data.ready_to_press = false;
                data.key = Some(KeyInput(*cmd.get_unchecked::<Input>(KEY_INPUT)));
            }
            _ => {}
        }
    }
}

struct HotKeySelector;

impl<W: Widget<UIState>> widget::Controller<UIState, W> for HotKeySelector {
    fn event(
        &mut self,
        _child: &mut W,
        _ctx: &mut EventCtx,
        event: &Event,
        data: &mut UIState,
        _env: &Env,
    ) {
        match event {
            Event::Command(cmd) if cmd.is(KEY_INPUT) => {
                let input = *cmd.get_unchecked::<Input>(KEY_INPUT);
                if data.ready_to_press_hotkey {
                    data.ready_to_press_hotkey = false;
                    data.hotkey = Some(KeyInput(input));
                } else if let Some(hotkey) = data.hotkey {
                    if data.on_off_state && hotkey.0 == input {
                        unsafe {
                            send_key_events(
                                data.macro_keys.iter().cloned(),
                                KeyState::Down,
                                &mut data.rng,
                            );
                            send_key_events(
                                data.macro_keys.iter().cloned(),
                                KeyState::Up,
                                &mut data.rng,
                            );
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

struct DelKeyDelegater;

impl AppDelegate<UIState> for DelKeyDelegater {
    fn command(
        &mut self,
        _ctx: &mut DelegateCtx,
        _target: Target,
        cmd: &Command,
        data: &mut UIState,
        _env: &Env,
    ) -> Handled {
        if let Some(target_key) = cmd.get(DEL_KEY) {
            let mut idx = 0;
            for (i, key) in data.macro_keys.iter().enumerate() {
                if key == target_key {
                    idx = i;
                    break;
                }
            }
            data.macro_keys.remove(idx);
            Handled::Yes
        } else {
            Handled::No
        }
    }
}
