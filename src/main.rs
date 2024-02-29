use std::{path::PathBuf, sync::Arc, time::Duration};

use hc_zome_profiles_integrity::Profile;
use hdk::prelude::ActionHash;
use holochain::start_happ;
use holomessage_integrity::HoloMessage;
use iced::{
    futures::SinkExt,
    widget::{column, row, text, text_input, Space, TextInput},
    Application, Color, Command, Length, Settings, Subscription, Theme,
};
use iced_holochain::happ::Happ;
use once_cell::sync::Lazy;

use crate::holochain::{create_message, create_profile, fetch_messages, fetch_profile};

static PROFILE_INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);
static MESSAGE_INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);

#[derive(Clone)]
enum Flags {
    HolochainDir(PathBuf),
}

impl Default for Flags {
    fn default() -> Self {
        let args: Vec<_> = std::env::args().collect();
        let mut holochain_dir =
            std::env::current_dir().expect("could not determine current directory");
        holochain_dir.push("holochain");
        if args.len() > 1 {
            if let Ok(path) = PathBuf::try_from(&args[1]) {
                holochain_dir.push(path);
            }
        }
        println!("conductor dir is {:?}", holochain_dir);
        Flags::HolochainDir(holochain_dir)
    }
}

fn main() -> iced::Result {
    let settings = Settings::<Flags>::default();
    Holomess::run(settings)
}

enum Holomess {
    Starting,
    Running(State),
    Failed(String),
}

struct State {
    happ: Arc<Happ>,
    error_message: Option<String>,
    nickname: String,
    profile: Option<Profile>,
    loading_profile: bool,
    loading_messages: bool,
    saving_message: bool,
    current_message: String,
    holo_messages: Vec<HoloMessage>,
}

impl State {
    pub fn new(happ: Arc<Happ>) -> Self {
        State {
            happ,
            error_message: None,
            nickname: String::new(),
            profile: None,
            loading_profile: false,
            loading_messages: false,
            saving_message: false,
            current_message: String::new(),
            holo_messages: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
enum Message {
    HappStarted(Result<Happ, String>),
    NicknameChanged(String),
    ProfileFetched(Result<Option<Profile>, String>),
    CreateProfile,
    ProfileCreated(Result<Profile, String>),
    HoloMessageChanged(String),
    CreateHoloMessage,
    HoloMessageCreated(Result<ActionHash, String>),
    HoloMessagesFetched(Result<Vec<HoloMessage>, String>),
}

impl Application for Holomess {
    type Message = Message;
    type Executor = iced::executor::Default;
    type Flags = Flags;
    type Theme = Theme;

    fn new(flags: Self::Flags) -> (Self, Command<Message>) {
        let holochain_dir = match flags {
            Flags::HolochainDir(holochain_dir) => holochain_dir,
        };
        (
            Holomess::Starting,
            Command::perform(start_happ(holochain_dir), Message::HappStarted),
        )
    }

    fn title(&self) -> String {
        "Holochain on iced".to_string()
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match self {
            Holomess::Starting => {
                println!("starting up HoloMessage...");
                let command = match message {
                    Message::HappStarted(Ok(happ)) => {
                        println!("happ started; fetching profile...");
                        let happ = Arc::new(happ);
                        let mut state = State::new(happ.clone());
                        state.loading_profile = true;
                        *self = Holomess::Running(state);
                        Command::perform(fetch_profile(happ), Message::ProfileFetched)
                    }
                    Message::HappStarted(Err(err)) => {
                        eprintln!("error starting happ - {err}");
                        *self = Holomess::Failed(err);
                        Command::none()
                    }
                    _ => Command::none(),
                };
                command
            }
            Holomess::Running(state) => {
                let command = match message {
                    Message::ProfileFetched(Ok(maybe_profile)) => {
                        state.loading_profile = false;
                        state.profile = maybe_profile;
                        if let Some(profile) = &state.profile {
                            println!("profile fetched is {profile:?}");
                            state.loading_messages = true;
                            Command::batch(vec![
                                // focus message input field
                                text_input::focus::<Message>(MESSAGE_INPUT_ID.clone()),
                                Command::perform(
                                    fetch_messages(state.happ.clone()),
                                    Message::HoloMessagesFetched,
                                ),
                            ])
                        } else {
                            println!("no profile found, need to create one");
                            text_input::focus::<Message>(PROFILE_INPUT_ID.clone())
                        }
                    }
                    Message::ProfileFetched(Err(err)) => {
                        state.loading_profile = false;
                        eprintln!("profile couldn't be fetched: {err}");
                        state.error_message = Some(format!("profile couldn't be fetched: {err}"));
                        Command::none()
                    }
                    Message::NicknameChanged(nickname) => {
                        state.nickname = nickname;
                        Command::none()
                    }
                    Message::HoloMessageChanged(message) => {
                        state.current_message = message;
                        Command::none()
                    }
                    Message::CreateHoloMessage => {
                        println!("creating new message {:?}", state.current_message);
                        state.saving_message = true;
                        Command::perform(
                            create_message(state.happ.clone(), state.current_message.clone()),
                            Message::HoloMessageCreated,
                        )
                    }
                    Message::HoloMessageCreated(Ok(message)) => {
                        state.saving_message = false;
                        println!("created a new message {message:?}, fetching all messages");
                        state.current_message = "".to_string();

                        state.loading_messages = true;
                        Command::batch(vec![
                            // focus message input field
                            text_input::focus::<Message>(MESSAGE_INPUT_ID.clone()),
                            Command::perform(
                                fetch_messages(state.happ.clone()),
                                Message::HoloMessagesFetched,
                            ),
                        ])
                    }
                    Message::HoloMessageCreated(Err(err)) => {
                        state.saving_message = false;
                        state.error_message = Some(err);
                        Command::none()
                    }
                    Message::CreateProfile => {
                        println!("creating profile for nickname {}", state.nickname);
                        Command::perform(
                            create_profile(state.happ.clone(), state.nickname.clone()),
                            Message::ProfileCreated,
                        )
                    }
                    Message::ProfileCreated(Ok(profile)) => {
                        println!("profile created: {profile:?}");
                        state.profile = Some(profile);
                        text_input::focus::<Message>(MESSAGE_INPUT_ID.clone())
                    }
                    Message::ProfileCreated(Err(err)) => {
                        eprintln!("error creating profile: {err}");
                        state.error_message = Some(format!("profile couldn't be created: {err}"));
                        Command::none()
                    }
                    Message::HoloMessagesFetched(Ok(holo_messages)) => {
                        state.loading_messages = false;
                        state.holo_messages = holo_messages;
                        Command::none()
                    }
                    Message::HoloMessagesFetched(Err(err)) => {
                        state.loading_messages = false;
                        state.error_message =
                            Some(format!("holo_messages couldn't be fetched: {err}"));
                        Command::none()
                    }
                    _ => Command::none(),
                };
                command
            }
            Holomess::Failed(err) => {
                eprintln!("starting HoloMessage failed: {err}");
                Command::none()
            }
        }
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        if let Holomess::Running(state) = self {
            if state.profile.is_some() {
                let happ = state.happ.clone();

                iced::subscription::channel(0u8, 100, |mut sender| async move {
                    loop {
                        let messages = fetch_messages(happ.clone()).await;
                        if let Err(err) = messages {
                            eprintln!("polling: could not fetch messages - {err}");
                        } else {
                            let _ = sender.send(Message::HoloMessagesFetched(messages)).await;
                        }
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                })
            } else {
                Subscription::none()
            }
        } else {
            Subscription::none()
        }
    }

    fn view(&self) -> iced::Element<'_, Self::Message> {
        let header = match self {
            Holomess::Starting => column![text("Starting up HoloMessage...")],
            Holomess::Running(state) => {
                if state.loading_profile {
                    column![text("Fetching profile...")]
                } else {
                    if let Some(profile) = &state.profile {
                        column![text(format!("Welcome back, {}", &profile.nickname)),]
                    } else {
                        let input = TextInput::new("Enter your nickname", &state.nickname)
                            .padding(10)
                            .on_input(Message::NicknameChanged)
                            .on_submit(Message::CreateProfile)
                            .id(PROFILE_INPUT_ID.clone());
                        column![text("First time here, let's create a profile:"), input]
                    }
                }
            }
            Holomess::Failed(err) => column![
                text("Starting Holomessage failed:"),
                text(err).style(Color::new(1.0, 0.0, 0.0, 1.0))
            ],
        };

        let mut mess_input = row![];
        if let Holomess::Running(state) = self {
            if state.profile.is_some() {
                let mut text_input = TextInput::new("Write a new message", &state.current_message)
                    .padding(10)
                    .on_submit(Message::CreateHoloMessage)
                    .id(MESSAGE_INPUT_ID.clone());
                if !state.saving_message {
                    text_input = text_input.on_input(Message::HoloMessageChanged);
                }
                mess_input = mess_input.push(text_input);
            }
        }

        let (holo_messages, error) = match self {
            Holomess::Running(state) => {
                if state.profile.is_some() {
                    // Messages header
                    let mut messages = column![text("Messages:")];

                    if state.loading_messages {
                        messages = messages.push("Loading messages...");
                    } else {
                        for holo_mess in state.holo_messages.iter() {
                            let mess_row = row![text(holo_mess.text.clone())];
                            messages = messages.push(mess_row);
                        }
                    }
                    let error = if let Some(error_message) = &state.error_message {
                        column![
                            text("An error happened:"),
                            text(error_message)
                                .style(iced::theme::Text::Color(Color::from_rgb(1.0, 0.0, 0.0)))
                        ]
                    } else {
                        column![]
                    };
                    (messages, error)
                } else {
                    (column![], column![])
                }
            }
            _ => (column![], column![]),
        };

        column![
            header,
            mess_input,
            holo_messages,
            Space::with_height(Length::Fill),
            error
        ]
        .width(Length::Fill)
        .padding(20)
        .spacing(20)
        .into()
    }
}

mod holochain {
    use crate::Happ;
    use hc_zome_profiles_integrity::Profile;
    use hdk::prelude::ActionHash;
    use holomessage_integrity::HoloMessage;
    use std::{path::PathBuf, sync::Arc};

    /// Spawn a Holochain conductor, install app and connect websockets to make
    /// requests to conductor.
    pub(crate) async fn start_happ(path: PathBuf) -> Result<Happ, String> {
        Happ::start_holochain_app(path).await
    }

    pub(crate) async fn fetch_profile(happ: Arc<Happ>) -> Result<Option<Profile>, String> {
        happ.fetch_profile(happ.cell_id.agent_pubkey().clone())
            .await
    }

    pub(crate) async fn create_profile(
        happ: Arc<Happ>,
        nickname: String,
    ) -> Result<Profile, String> {
        happ.create_profile(nickname).await
    }

    pub(crate) async fn create_message(
        happ: Arc<Happ>,
        message: String,
    ) -> Result<ActionHash, String> {
        happ.create_message(message).await
    }

    pub(crate) async fn fetch_messages(happ: Arc<Happ>) -> Result<Vec<HoloMessage>, String> {
        happ.fetch_messages().await
    }
}
