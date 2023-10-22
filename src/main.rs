use std::{path::Path, sync::Arc};

use get_port::Ops;
use holochain::{
    conductor::{
        config::{AdminInterfaceConfig, ConductorConfig, KeystoreConfig},
        interface::InterfaceDriver,
        paths::DatabaseRootPath,
        ConductorBuilder,
    },
    prelude::kitsune_p2p::dependencies::{
        kitsune_p2p_types::config::{KitsuneP2pConfig, TransportConfig},
        url2::url2,
    },
};
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
                Command::perform(connect(), |message| message)
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

pub fn vec_to_locked(mut pass_tmp: Vec<u8>) -> std::io::Result<sodoken::BufRead> {
    match sodoken::BufWrite::new_mem_locked(pass_tmp.len()) {
        Err(e) => {
            pass_tmp.fill(0);
            Err(e.into())
        }
        Ok(p) => {
            {
                let mut lock = p.write_lock();
                lock.copy_from_slice(&pass_tmp);
                pass_tmp.fill(0);
            }
            Ok(p.to_read())
        }
    }
}

async fn connect() -> Message {
    let mut conductor_config = ConductorConfig::default();
    conductor_config.environment_path = DatabaseRootPath::from(Path::new(
        "/Users/jost/Desktop/dev/rust/iced-holochain/conductor",
    ));
    conductor_config.keystore = KeystoreConfig::LairServerInProc {
        lair_root: Some(
            Path::new("/Users/jost/Desktop/dev/rust/iced-holochain/lair").to_path_buf(),
        ),
    };
    let admin_port = if let Some(port) = get_port::tcp::TcpPort::in_range(
        "127.0.0.1",
        get_port::Range {
            min: 1,
            max: 2,
            // min: 55000,
            // max: 56000,
        },
    ) {
        port
    } else {
        eprintln!("couldn't get a free port for admin interface");
        return Message::Error(Error::HolochainError(
            "couldn't get a free port for admin interface".to_string(),
        ));
    };
    let admin_interface_config = AdminInterfaceConfig {
        driver: InterfaceDriver::Websocket { port: admin_port },
    };

    let mut network_config = KitsuneP2pConfig::default();
    network_config.bootstrap_service = Some(url2!("https://bootstrap.holo.host"));
    network_config.transport_pool.push(TransportConfig::WebRTC {
        signal_url: "wss://signal.holo.host".to_string(),
    });
    conductor_config.network = Some(network_config);

    conductor_config.admin_interfaces = Some(vec![admin_interface_config]);

    match ConductorBuilder::new()
        .config(conductor_config)
        .passphrase(Some(vec_to_locked("pass".as_bytes().to_owned()).unwrap()))
        .build()
        .await
    {
        Err(err) => {
            eprintln!("building conductor failed {:#?}", err);
            Message::Error(Error::HolochainError(format!(
                "building conductor failed {:#?}",
                err
            )))
        }
        Ok(conductor) => {
            println!("conductor built: config {:?}", conductor.config);
            if let Some(admin_port) = conductor.get_arbitrary_admin_websocket_port() {
                match AdminWebsocket::connect(format!("ws://127.0.0.1:{}", admin_port)).await {
                    Ok(admin_ws) => {
                        println!("admin web soccket connected");
                        Message::AdminWsConnected(Arc::new(admin_ws))
                    }
                    Err(err) => {
                        eprintln!("failed to connect admin web socket {:?}", err);
                        Message::Error(Error::HolochainError(format!(
                            "failed to connect admin web socket {:?}",
                            err
                        )))
                    }
                }
            } else {
                Message::Error(Error::HolochainError(
                    "could not get admin port".to_string(),
                ))
            }
        }
    }
}
