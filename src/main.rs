use std::sync::Arc;

use holochain::spawn_app;
use holochain_client::AdminWebsocket;
use iced::{
    widget::{button, column, text, Space},
    Application, Color, Command, Length, Settings, Theme,
};

fn main() -> iced::Result {
    Happ::run(Settings {
        ..Settings::default()
    })
}

struct Happ {
    admin_ws: Option<Arc<AdminWebsocket>>,
    holochain_starting: bool,
    error: String,
}

#[derive(Clone, Debug)]
enum Message {
    ButtonPressed,
    AdminWsConnected(Arc<AdminWebsocket>),
    Error(Error),
}

#[derive(Clone, Debug)]
enum Error {
    HolochainError(String),
}

impl Application for Happ {
    type Message = Message;
    type Executor = iced::executor::Default;
    type Flags = ();
    type Theme = Theme;

    fn new(_: Self::Flags) -> (Self, Command<Message>) {
        (
            Happ {
                admin_ws: None,
                holochain_starting: false,
                error: String::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Gherkin".to_string()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::ButtonPressed => {
                self.holochain_starting = true;
                Command::perform(spawn_app(), |message| message)
            }
            Message::AdminWsConnected(admin_ws) => {
                println!("admin ws connected");
                self.admin_ws = Some(admin_ws);
                self.holochain_starting = false;
                Command::none()
            }
            Message::Error(err) => {
                match err {
                    Error::HolochainError(e) => self.error = e,
                }
                Command::none()
            }
        }
    }

    fn view(&self) -> iced::Element<'_, Self::Message> {
        let holochain_starting_text = if self.holochain_starting {
            "Holochain is starting"
        } else {
            "Holochain is not starting"
        };
        let btn_text = if self.admin_ws.is_some() {
            "connected"
        } else {
            "not connected"
        };
        let error_text = self.error.clone();
        column![
            button("batn").padding(10).on_press(Message::ButtonPressed),
            text(format!("{}", holochain_starting_text)),
            text(format!("{}", btn_text)),
            Space::new(Length::Fill, Length::Fill),
            text(error_text).style(Color::new(1.0, 0.0, 0.0, 1.0)),
        ]
        .width(Length::Fill)
        .align_items(iced::Alignment::Center)
        .padding(20)
        .spacing(20)
        .into()
    }
}

mod holochain {
    use crate::{Error, Message};
    use iced_holochain::spawn_holochain_app;

    pub(crate) async fn spawn_app() -> Message {
        match spawn_holochain_app().await {
            Err(err) => Message::Error(Error::HolochainError(err)),
            Ok(admin_ws) => Message::AdminWsConnected(admin_ws),
        }
    }
}
