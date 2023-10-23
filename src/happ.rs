use std::{fmt::Debug, sync::Arc};

use get_port::Ops;
use holochain::{
    conductor::{
        config::{AdminInterfaceConfig, ConductorConfig, KeystoreConfig},
        interface::InterfaceDriver,
        paths::DatabaseRootPath,
        Conductor, ConductorBuilder,
    },
    prelude::kitsune_p2p::dependencies::{
        kitsune_p2p_types::config::{KitsuneP2pConfig, TransportConfig},
        url2::url2,
    },
};
use holochain_client::AdminWebsocket;

#[derive(Clone)]
pub struct Happ {
    pub conductor: Arc<Conductor>,
    pub admin_ws: Arc<AdminWebsocket>,
}

impl Debug for Happ {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Happ").finish()
    }
}

pub async fn start_holochain_app() -> Result<Happ, String> {
    let temp_dir = tempfile::tempdir().map_err(|err| err.to_string())?;
    let mut conductor_config = ConductorConfig::default();
    conductor_config.environment_path = DatabaseRootPath::from(temp_dir.path().join("conductor/"));
    conductor_config.keystore = KeystoreConfig::LairServerInProc {
        lair_root: Some(temp_dir.path().join("lair/")),
    };
    let admin_port = if let Some(port) = get_port::tcp::TcpPort::in_range(
        "127.0.0.1",
        get_port::Range {
            min: 55000,
            max: 56000,
        },
    ) {
        port
    } else {
        eprintln!("couldn't get a free port for admin interface");
        return Err("couldn't get a free port for admin interface".to_string());
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

    println!("temp dir {:?}", temp_dir.into_path());

    match ConductorBuilder::new()
        .config(conductor_config)
        .passphrase(Some(vec_to_locked("pass".as_bytes().to_owned()).unwrap()))
        .build()
        .await
    {
        Err(err) => {
            eprintln!("building conductor failed {:#?}", err);
            Err(format!("building conductor failed {:#?}", err))
        }
        Ok(conductor) => {
            println!("conductor built: config {:?}", conductor.config);
            if let Some(admin_port) = conductor.get_arbitrary_admin_websocket_port() {
                match AdminWebsocket::connect(format!("ws://127.0.0.1:{}", admin_port)).await {
                    Ok(admin_ws) => {
                        println!("admin web soccket connected");
                        let happ = Happ {
                            conductor,
                            admin_ws: Arc::new(admin_ws),
                        };
                        Ok(happ)
                    }
                    Err(err) => {
                        eprintln!("failed to connect admin web socket {:?}", err);
                        Err(format!("failed to connect admin web socket {:?}", err))
                    }
                }
            } else {
                Err("could not get admin port".to_string())
            }
        }
    }
}

fn vec_to_locked(mut pass_tmp: Vec<u8>) -> std::io::Result<sodoken::BufRead> {
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
