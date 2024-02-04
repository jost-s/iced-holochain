use std::{path::PathBuf, sync::Arc};

use hc_zome_profiles_integrity::Profile;
use hdk::prelude::ActionHash;
use holochain::start_happ;
use holomess_integrity::HoloMess;
use iced::{
    widget::{column, row, text, text_input, Space, TextInput},
    Application, Color, Command, Length, Settings, Theme,
};
use iced_holochain::happ::Happ;
use once_cell::sync::Lazy;

use crate::holochain::{create_message, create_profile, fetch_messages, fetch_profile};

static INPUT_ID: Lazy<text_input::Id> = Lazy::new(text_input::Id::unique);

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
        println!("conductor dir {:?}", holochain_dir);
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
    holo_messes: Vec<HoloMess>,
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
            holo_messes: Vec::new(),
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
    HoloMessChanged(String),
    CreateHoloMess,
    HoloMessCreated(Result<ActionHash, String>),
    HoloMessesFetched(Result<Vec<HoloMess>, String>),
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
                println!("starting up holomess");
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
                        eprintln!("err is {err}");
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
                            let _ = text_input::focus::<Message>(INPUT_ID.clone());
                            state.loading_messages = true;
                            Command::perform(
                                fetch_messages(state.happ.clone()),
                                Message::HoloMessesFetched,
                            )
                        } else {
                            println!("no profile found, need to create one");
                            Command::none()
                        }
                    }
                    Message::ProfileFetched(Err(err)) => {
                        state.loading_profile = false;
                        eprintln!("oh shit, profile couldn't be fetched: {err}");
                        state.error_message =
                            Some(format!("oh shit, profile couldn't be fetched: {err}"));
                        Command::none()
                    }
                    Message::NicknameChanged(nickname) => {
                        state.nickname = nickname;
                        Command::none()
                    }
                    Message::HoloMessChanged(message) => {
                        state.current_message = message;
                        Command::none()
                    }
                    Message::CreateHoloMess => {
                        println!("creating new mess {:?}", state.current_message);
                        state.saving_message = true;
                        Command::perform(
                            create_message(state.happ.clone(), state.current_message.clone()),
                            Message::HoloMessCreated,
                        )
                    }
                    Message::HoloMessCreated(Ok(mess)) => {
                        state.saving_message = false;
                        println!("created a new mess {mess:?}, fetching all messes");
                        state.current_message = "".to_string();

                        state.loading_messages = true;
                        Command::perform(
                            fetch_messages(state.happ.clone()),
                            Message::HoloMessesFetched,
                        )
                    }
                    Message::HoloMessCreated(Err(err)) => {
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
                        println!("another output");
                        Command::none()
                    }
                    Message::ProfileCreated(Err(err)) => {
                        eprintln!("oh shit, this is heavy: {err}");
                        state.error_message =
                            Some(format!("oh shit, profile couldn't be created: {err}"));
                        Command::none()
                    }
                    Message::HoloMessesFetched(Ok(holo_messes)) => {
                        state.loading_messages = false;
                        println!("holomesses fetched; there are {:?}", holo_messes.len());
                        state.holo_messes = holo_messes;
                        Command::none()
                    }
                    Message::HoloMessesFetched(Err(err)) => {
                        state.loading_messages = false;
                        state.error_message =
                            Some(format!("oh shit, holo_messes couldn't be fetched: {err}"));
                        Command::none()
                    }
                    _ => Command::none(),
                };
                command
            }
            Holomess::Failed(err) => {
                eprintln!("starting holomess failed: {err}");
                Command::none()
            }
        }
    }

    fn view(&self) -> iced::Element<'_, Self::Message> {
        let header = match self {
            Holomess::Starting => column![text("Starting up holomess...")],
            Holomess::Running(state) => {
                if state.loading_profile {
                    column![text("Fetching profile...")]
                } else {
                    if let Some(profile) = &state.profile {
                        println!("found profile, showing welcome");
                        column![text(format!("Welcome back, {}", &profile.nickname)),]
                    } else {
                        let input = TextInput::new("Enter your nickname", &state.nickname)
                            .padding(10)
                            .on_input(Message::NicknameChanged)
                            .on_submit(Message::CreateProfile);
                        column![text("First time here, let's create a profile:"), input]
                    }
                }
            }
            Holomess::Failed(err) => column![
                text("Starting Holomess failed:"),
                text(err).style(Color::new(1.0, 0.0, 0.0, 1.0))
            ],
        };

        let mut mess_input = row![];
        if let Holomess::Running(state) = self {
            if state.profile.is_some() {
                let mut text_input = TextInput::new("Write a new message", &state.current_message)
                    .padding(10)
                    .on_submit(Message::CreateHoloMess)
                    .id(INPUT_ID.clone());
                if !state.saving_message {
                    text_input = text_input.on_input(Message::HoloMessChanged);
                }
                mess_input = mess_input.push(text_input);
            }
        }

        let (holo_messes, error) = match self {
            Holomess::Running(state) => {
                if state.profile.is_some() {
                    // Messages header
                    let mut messes = column![text("Messes:")];

                    if state.loading_messages {
                        messes = messes.push("Loading messages...");
                    } else {
                        for holo_mess in state.holo_messes.iter() {
                            let mess_row = row![text(holo_mess.text.clone())];
                            messes = messes.push(mess_row);
                        }
                    }
                    let error = if let Some(error_message) = &state.error_message {
                        column![
                            text("A big bad error happened"),
                            text(error_message)
                                .style(iced::theme::Text::Color(Color::from_rgb(1.0, 0.0, 0.0)))
                        ]
                    } else {
                        column![]
                    };
                    (messes, error)
                } else {
                    (column![], column![])
                }
            }
            _ => (column![], column![]),
        };

        column![
            header,
            mess_input,
            holo_messes,
            Space::with_height(Length::Fill),
            error
        ]
        .width(Length::Fill)
        // .align_items(iced::Alignment::Center)
        .padding(20)
        .spacing(20)
        .into()
    }
}

mod holochain {
    use crate::Happ;
    use hc_zome_profiles_integrity::Profile;
    use hdk::prelude::ActionHash;
    use holomess_integrity::HoloMess;
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

    pub(crate) async fn fetch_messages(happ: Arc<Happ>) -> Result<Vec<HoloMess>, String> {
        println!("fetching holo_messes...");
        happ.fetch_messages().await
    }
}
