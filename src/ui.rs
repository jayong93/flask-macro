use crate::send_key_events;
use crate::AutoKey;
use crate::Receiver;
use crate::{Input, KeyState};
use iced::*;
use iced_native::subscription::Recipe;
use rand::prelude::*;

#[derive(Debug, Clone)]
pub enum UIMessage {
    KeyEvent((Input, KeyState)),
}

pub struct UIState {
    macro_keys: Vec<AutoKey>,
    hotkey: Option<Input>,
    key_receiver: *mut Receiver,
    rng: SmallRng,
}

impl Application for UIState {
    type Executor = iced::executor::Default;
    type Message = UIMessage;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Self::Message>) {
        (
            Self {
                macro_keys: vec![],
                hotkey: Some(Input::KeyBoard(1)),
                key_receiver: Box::leak(Box::new(Receiver::new())),
                rng: SmallRng::from_entropy(),
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
        match message {
            UIMessage::KeyEvent((key, state)) => {
                eprintln!("{}, {:#?}", key, state);
                if let Some(hotkey) = self.hotkey {
                    if key == hotkey {
                        unsafe { send_key_events(&self.macro_keys, state, &mut self.rng) }
                    }
                }
            }
        }
        Command::none()
    }

    fn view(&mut self) -> Element<'_, Self::Message> {
        Column::new().into()
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
