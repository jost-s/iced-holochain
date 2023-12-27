use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

use hc_zome_profiles_integrity::Profile;
use holochain::start_happ;
use iced::{
    widget::{column, text, TextInput},
    Application, Color, Command, Length, Settings, Theme,
};
use iced_holochain::happ::Happ;

use crate::holochain::{create_profile, fetch_profile};

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

enum Loading {
    FetchingProfile,
    None,
}

struct State {
    happ: Arc<Happ>,
    nickname: String,
    loading: Loading,
    profile: Option<Profile>,
}

impl State {
    pub fn new(happ: Arc<Happ>) -> Self {
        State {
            nickname: String::new(),
            profile: None,
            loading: Loading::None,
            happ,
        }
    }
}

#[derive(Clone, Debug)]
enum Message {
    HappStarted(Result<Happ, String>),
    NicknameChanged(String),
    ProfileFetched(Result<Profile, String>),
    CreateProfile,
    ProfileCreated(Result<Profile, String>),
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
                        state.loading = Loading::FetchingProfile;
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
                    Message::ProfileFetched(Ok(profile)) => {
                        println!("profile fetched is {profile:?}");
                        state.profile = Some(profile);
                        state.loading = Loading::None;
                        Command::none()
                    }
                    Message::ProfileFetched(Err(err)) => {
                        eprintln!("oh shit, profile couldn't be fetched: {err}");
                        state.loading = Loading::None;
                        Command::none()
                    }
                    Message::NicknameChanged(nickname) => {
                        state.nickname = nickname;
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
                        Command::none()
                    }
                    Message::ProfileCreated(Err(err)) => {
                        eprintln!("oh shit, this is heavy: {err}");
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
        let content = match self {
            Holomess::Starting => column![text("starting up holomess...")],
            Holomess::Running(state) => {
                if let Loading::FetchingProfile = &state.loading {
                    column![text("Fetching profile...")]
                } else {
                    if let Some(profile) = &state.profile {
                        column![text(format!("Welcome back, {}", &profile.nickname))]
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
        content
            .width(Length::Fill)
            .align_items(iced::Alignment::Center)
            .padding(20)
            .spacing(20)
            .into()
    }
}

mod holochain {
    use crate::Happ;
    use hc_zome_profiles_integrity::Profile;
    use std::{path::PathBuf, sync::Arc};

    /// Spawn a Holochain conductor, install app and connect websockets to make
    /// requests to conductor.
    pub(crate) async fn start_happ(path: PathBuf) -> Result<Happ, String> {
        Happ::start_holochain_app(path).await
    }

    pub(crate) async fn fetch_profile(happ: Arc<Happ>) -> Result<Profile, String> {
        happ.fetch_profile(happ.cell_id.agent_pubkey().clone())
            .await
    }

    pub(crate) async fn create_profile(
        happ: Arc<Happ>,
        nickname: String,
    ) -> Result<Profile, String> {
        happ.create_profile(nickname).await
    }
}
