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

pub async fn spawn_holochain_app() -> Result<Arc<AdminWebsocket>, String> {
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
                        Ok(Arc::new(admin_ws))
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
