use holochain::start_happ;
use iced::{
    widget::{button, column, text, Space},
    Application, Color, Command, Length, Settings, Theme,
};
use iced_holochain::happ::Happ;

fn main() -> iced::Result {
    State::run(Settings {
        ..Settings::default()
    })
}
struct State {
    happ: Option<Happ>,
    holochain_starting: bool,
    error: String,
}

#[derive(Clone, Debug)]
enum Message {
    ButtonPressed,
    HappStarted(Happ),
    Error(Error),
}

#[derive(Clone, Debug)]
enum Error {
    HolochainError(String),
}

impl Application for State {
    type Message = Message;
    type Executor = iced::executor::Default;
    type Flags = ();
    type Theme = Theme;

    fn new(_: Self::Flags) -> (Self, Command<Message>) {
        (
            State {
                happ: None,
                holochain_starting: false,
                error: String::new(),
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        "Holochain on iced".to_string()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::ButtonPressed => {
                self.holochain_starting = true;
                Command::perform(start_happ(), |message| message)
            }
            Message::HappStarted(admin_ws) => {
                println!("admin ws connected");
                self.happ = Some(admin_ws);
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
        let btn_text = if self.happ.is_some() {
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
    use iced_holochain::happ::start_holochain_app;

    /// Spawn a Holochain conductor, install app and connect websockets to make
    /// requests to app.
    pub(crate) async fn start_happ() -> Message {
        match start_holochain_app().await {
            Err(err) => Message::Error(Error::HolochainError(err)),
            Ok(admin_ws) => Message::HappStarted(admin_ws),
        }
    }
}
