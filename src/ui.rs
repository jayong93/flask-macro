use crate::send_key_events;
use crate::AutoKey;
use crate::Receiver;
use crate::{Input, KeyState};
use iced::*;
use iced_native::subscription::Recipe;
use rand::prelude::*;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum UIMessage {
    KeyEvent((Input, KeyState)),
    InputDelay(String),
    Apply,
    AddKey,
    EditKey(usize),
    EditDelay(usize),
    EditHotkey,
    Delete(usize),
}

use std::cell::UnsafeCell;
enum UISubState {
    Normal,
    AddKey {
        key: Option<Input>,
        delay: String,
        input_state: UnsafeCell<text_input::State>,
    },
    EditKey {
        key_id: usize,
    },
    EditDelay {
        key_id: usize,
        input_string: String,
    },
    EditHotkey,
}

struct AutoKeyState(
    AutoKey,
    text_input::State,
    button::State,
    button::State,
    button::State,
);

pub struct UIState {
    macro_keys: Vec<AutoKeyState>,
    hotkey: Option<Input>,
    key_receiver: *mut Receiver,
    rng: SmallRng,
    sub_state: UISubState,
    scroll_state: scrollable::State,
    add_key_button_state: button::State,
    edit_hotkey_button_state: button::State,
}

impl Application for UIState {
    type Executor = iced::executor::Default;
    type Message = UIMessage;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                macro_keys: vec![],
                hotkey: None,
                key_receiver: Box::leak(Box::new(Receiver::new())),
                rng: SmallRng::from_entropy(),
                sub_state: UISubState::Normal,
                scroll_state: scrollable::State::new(),
                add_key_button_state: button::State::new(),
                edit_hotkey_button_state: button::State::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "POE Flask Macro".to_string()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        iced_native::subscription::Subscription::from_recipe(KeyReceiver(
            self.key_receiver as usize,
        ))
        .map(UIMessage::KeyEvent)
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match &mut self.sub_state {
            UISubState::Normal => match message {
                UIMessage::AddKey => {
                    self.sub_state = UISubState::AddKey {
                        key: None,
                        delay: String::new(),
                        input_state: UnsafeCell::new(text_input::State::new()),
                    };
                }
                UIMessage::EditKey(idx) => {
                    self.sub_state = UISubState::EditKey { key_id: idx };
                }
                UIMessage::EditDelay(idx) => {
                    self.sub_state = UISubState::EditDelay {
                        key_id: idx,
                        input_string: String::new(),
                    };
                }
                UIMessage::EditHotkey => {
                    self.sub_state = UISubState::EditHotkey;
                }
                UIMessage::KeyEvent((key, state)) => {
                    if let Some(hotkey) = self.hotkey {
                        if key == hotkey {
                            let v = self.macro_keys.iter().map(|AutoKeyState(key, ..)| *key);
                            unsafe { send_key_events(v, state, &mut self.rng) }
                        }
                    }
                }
                UIMessage::Delete(idx) => {
                    self.macro_keys.remove(idx);
                }
                _ => {}
            },
            UISubState::EditKey { key_id } => match message {
                UIMessage::KeyEvent((key, state)) if state == KeyState::Down => {
                    self.macro_keys[*key_id].0.key = key;
                    self.sub_state = UISubState::Normal;
                }
                _ => {}
            },
            UISubState::EditDelay {
                key_id,
                input_string,
            } => match message {
                UIMessage::InputDelay(delay_string) => {
                    *input_string = delay_string;
                }
                UIMessage::Apply => {
                    if let Ok(delay) = input_string.parse::<f64>() {
                        self.macro_keys[*key_id].0.delay = Duration::from_secs_f64(delay);
                    }
                    self.sub_state = UISubState::Normal;
                }
                _ => {}
            },
            UISubState::EditHotkey => {
                if let UIMessage::KeyEvent((key, state)) = message {
                    if state == KeyState::Down {
                        self.hotkey = Some(key);
                        self.sub_state = UISubState::Normal;
                    }
                }
            }
            UISubState::AddKey { key, delay, .. } => match message {
                UIMessage::KeyEvent((key_, state)) if state == KeyState::Down => {
                    if let None = key {
                        *key = Some(key_);
                    }
                }
                UIMessage::InputDelay(delay_string) => {
                    *delay = delay_string;
                }
                UIMessage::Apply => {
                    if let Some(key) = key {
                        if let Ok(delay) = delay.parse() {
                            self.macro_keys.push(AutoKeyState(
                                AutoKey {
                                    key: *key,
                                    delay: Duration::from_secs_f64(delay),
                                },
                                text_input::State::new(),
                                button::State::new(),
                                button::State::new(),
                                button::State::new(),
                            ));
                        }
                    }
                    self.sub_state = UISubState::Normal;
                }
                _ => {}
            },
        }
        Command::none()
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        let mut scroll = Scrollable::new(&mut self.scroll_state)
            .align_items(Align::Center)
            .spacing(20);
        let sub_state = &mut self.sub_state;
        for (
            idx,
            AutoKeyState(auto_key, input_state, button_state1, button_state2, button_state3),
        ) in self.macro_keys.iter_mut().enumerate()
        {
            let row = Row::new().spacing(20);
            let row: Element<Self::Message> = match sub_state {
                UISubState::EditDelay {
                    key_id,
                    input_string,
                } if *key_id == idx => row
                    .push(
                        Text::new(format!("Key: {}", auto_key.key.to_string()))
                            .width(Length::Fill)
                            .horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .push(
                        TextInput::new(input_state, "Delay", &input_string, UIMessage::InputDelay)
                            .width(Length::Fill)
                            .on_submit(UIMessage::Apply),
                    ),
                UISubState::EditKey { key_id } if *key_id == idx => row
                    .push(
                        Text::new("Press new key")
                            .horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .push(
                        Text::new(format!(
                            "Delay: {} s",
                            auto_key.delay.as_secs_f64().to_string()
                        ))
                        .horizontal_alignment(HorizontalAlignment::Center),
                    ),
                _ => row
                    .push(
                        Text::new(format!("Key: {}", auto_key.key.to_string()))
                            .width(Length::Fill)
                            .horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .push(
                        Button::new(button_state1, Text::new("Edit"))
                            .width(Length::Shrink)
                            .on_press(UIMessage::EditKey(idx)),
                    )
                    .push(
                        Text::new(format!(
                            "Delay: {} s",
                            auto_key.delay.as_secs_f64().to_string()
                        ))
                        .width(Length::Fill)
                        .horizontal_alignment(HorizontalAlignment::Center),
                    )
                    .push(
                        Button::new(button_state2, Text::new("Edit"))
                            .width(Length::Shrink)
                            .on_press(UIMessage::EditDelay(idx)),
                    )
                    .push(Space::with_width(Length::Units(30)))
                    .push(
                        Button::new(button_state3, Text::new("Delete"))
                            .width(Length::Shrink)
                            .on_press(UIMessage::Delete(idx)),
                    ),
            }
            .into();
            scroll = scroll.push(row);
        }

        if let UISubState::AddKey {
            key,
            delay,
            input_state,
        } = &self.sub_state
        {
            let mut row = Row::new().spacing(20);
            match (key, delay) {
                (None, _) => {
                    row = row.push(Text::new("Press Macro Key"));
                }
                (Some(input), ref delay) => {
                    row = row
                        .push(
                            Text::new(format!("Key: {}", input.to_string()))
                                .width(Length::Fill)
                                .horizontal_alignment(HorizontalAlignment::Center),
                        )
                        .push(
                            TextInput::new(
                                unsafe { &mut *input_state.get() },
                                "Delay",
                                delay,
                                UIMessage::InputDelay,
                            )
                            .width(Length::Fill)
                            .on_submit(UIMessage::Apply),
                        );
                }
            }
            scroll = scroll.push(row);
        }

        let buttons = Row::new()
            .align_items(Align::Center)
            .push(
                Button::new(&mut self.add_key_button_state, Text::new("Add Macro Key"))
                    .width(Length::Shrink)
                    .height(Length::Shrink)
                    .on_press(UIMessage::AddKey),
            )
            .push(
                Button::new(
                    &mut self.edit_hotkey_button_state,
                    Text::new("Change Hotkey"),
                )
                .on_press(UIMessage::EditHotkey),
            );

        Container::new(
            Column::new()
                .align_items(Align::Center)
                .spacing(50)
                .push(scroll.width(Length::Fill).height(Length::Fill))
                .push(Text::new(if let UISubState::EditHotkey = self.sub_state {
                    "Press Hotkey".to_string()
                } else {
                    format!(
                        "Hotkey : {}",
                        self.hotkey
                            .map_or_else(|| "None".to_string(), |key| key.to_string())
                    )
                }))
                .push(buttons),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x()
        .center_y()
        .into()
    }
}

use iced::futures::stream::{self, *};
use iced::futures::{Future, FutureExt};

#[derive(Debug, Clone)]
struct KeyReceiver(usize);

impl KeyReceiver {
    unsafe fn get<'a>(&self) -> impl Future<Output = Option<(Input, KeyState)>> + 'a {
        let receiver: &'a mut _;
        {
            receiver = &mut *(self.0 as *mut Receiver);
        }
        receiver.get()
    }
}

impl<H, I> Recipe<H, I> for KeyReceiver
where
    H: std::hash::Hasher,
{
    type Output = (Input, KeyState);

    fn hash(&self, state: &mut H) {
        use std::hash::Hash;

        std::any::TypeId::of::<Self>().hash(state);
    }

    fn stream(self: Box<Self>, _input: BoxStream<'static, I>) -> BoxStream<'static, Self::Output> {
        unsafe { stream::unfold((), move |_| self.get().map(|v| v.map(|v| (v, ())))).boxed() }
    }
}
