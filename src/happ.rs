use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    path::{Path, PathBuf},
    sync::Arc,
};

use get_port::Ops;
use hc_zome_profiles_integrity::Profile;
use hdk::prelude::{CellId, ExternIO, FunctionName, Record, Timestamp, ZomeName};
use holochain::{
    conductor::{
        api::CellInfo,
        config::{AdminInterfaceConfig, ConductorConfig, KeystoreConfig},
        interface::InterfaceDriver,
        paths::DatabaseRootPath,
        Conductor, ConductorBuilder,
    },
    prelude::{
        kitsune_p2p::dependencies::url2::url2, AppBundleSource, KitsuneP2pConfig, TransportConfig,
        ZomeCallUnsigned,
    },
};
use holochain_client::{AdminWebsocket, AgentPubKey, AppWebsocket, InstallAppPayload, ZomeCall};
use holochain_state::nonce::fresh_nonce;
use serde::{de::DeserializeOwned, Serialize};

const CONDUCTOR_CONFIG_FILE: &str = "conductor-config.yaml";

#[derive(Clone)]
pub struct Happ {
    pub conductor: Arc<Conductor>,
    pub admin_ws: Arc<AdminWebsocket>,
    pub app_ws: Arc<AppWebsocket>,
    pub cell_id: Arc<CellId>,
}

impl Debug for Happ {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Happ").finish()
    }
}

impl Happ {
    pub async fn start_holochain_app(path: PathBuf) -> Result<Happ, String> {
        let conductor_config_path = path.join(CONDUCTOR_CONFIG_FILE);
        let conductor_config = if conductor_config_path.exists() {
            println!("conductor exists");
            ConductorConfig::load_yaml(&conductor_config_path)
                .map_err(|err| format!("error loading conductor config from file: {:?}", err))?
        } else {
            println!("creating new conductor");

            let mut conductor_config = ConductorConfig::default();
            conductor_config.environment_path = DatabaseRootPath::from(path.join("conductor"));
            conductor_config.keystore = KeystoreConfig::LairServerInProc {
                lair_root: Some(path.join("lair")),
            };
            let admin_port = get_port::tcp::TcpPort::in_range(
                "127.0.0.1",
                get_port::Range {
                    min: 64000,
                    max: 65000,
                },
            )
            .ok_or_else(|| {
                eprintln!("couldn't get a free port for admin interface");
                "couldn't get a free port for admin interface".to_string()
            })?;
            let admin_interface_config = AdminInterfaceConfig {
                driver: InterfaceDriver::Websocket { port: admin_port },
            };
            conductor_config.admin_interfaces = Some(vec![admin_interface_config]);

            let mut network_config = KitsuneP2pConfig::default();
            network_config.bootstrap_service = Some(url2!("https://bootstrap.holo.host"));
            network_config.transport_pool.push(TransportConfig::WebRTC {
                signal_url: "wss://signal.holo.host".to_string(),
            });
            conductor_config.network = Some(network_config);
            println!("conductor config is {:?}", conductor_config);

            let conductor_config_contents = serde_yaml::to_string(&conductor_config)
                .map_err(|err| format!("could not serialize conductor config: {:?}", err))?;
            println!(
                "conductor config string is is {}",
                conductor_config_contents
            );

            std::fs::create_dir_all(path)
                .map_err(|err| format!("could not create holochain directory: {:?}", err))?;
            std::fs::write(conductor_config_path, conductor_config_contents)
                .map_err(|err| format!("writing conductor config to file failed: {:?}", err))?;

            conductor_config
        };

        let conductor = ConductorBuilder::default()
            .config(conductor_config.clone())
            .passphrase(Some(vec_to_locked("pass".as_bytes().to_owned()).unwrap()))
            .build()
            .await
            .map_err(|err| format!("building conductor failed {:#?}", err))?;
        println!("conductor built: config {:?}", conductor.config);

        let admin_port = match conductor_config
            .admin_interfaces
            .expect("no admin interfaces in conductor config")[0]
            .driver
        {
            InterfaceDriver::Websocket { port } => port,
        };
        let mut admin_ws = AdminWebsocket::connect(format!("ws://127.0.0.1:{}", admin_port))
            .await
            .map_err(|err| format!("failed to connect admin web socket {:?}", err))?;
        println!("admin web socket connected");

        let mut installed_apps = conductor
            .list_apps(None)
            .await
            .map_err(|err| format!("could not query apps: {:?}", err))?;
        let cell_info = if installed_apps.len() == 0 {
            println!("no installed apps found - installing...");
            let agent_key = admin_ws
                .generate_agent_pub_key()
                .await
                .map_err(|err| format!("error generating an agent pub key: {:?}", err))?;
            println!("generated agent pub key {:?}", agent_key);

            let install_app_payload = InstallAppPayload {
                source: AppBundleSource::Path(
                    Path::new("happ/workdir/holomess.happ").to_path_buf(),
                ),
                agent_key: agent_key.clone(),
                installed_app_id: None,
                membrane_proofs: HashMap::new(),
                network_seed: None,
            };
            let app_info = admin_ws
                .install_app(install_app_payload)
                .await
                .map_err(|err| format!("error installing app: {:?}", err))?;
            println!("app installed {:?}", app_info);

            admin_ws
                .enable_app(app_info.installed_app_id.clone())
                .await
                .map_err(|err| format!("error enabling app: {:?}", err))?;
            println!("app enabled");

            app_info.cell_info
        } else {
            println!("found installed app");
            let app = installed_apps.pop().unwrap();
            // start app if it is not yet running
            match app.status {
                holochain::conductor::api::AppInfoStatus::Running => (),
                _ => {
                    println!("installed apps not running - enabling...");
                    admin_ws
                        .enable_app(app.installed_app_id.to_owned())
                        .await
                        .map_err(|err| format!("could not enable app: {:?}", err))?;
                }
            }

            app.cell_info
        };

        let cells = cell_info.get("holomess").ok_or_else(|| "cell not found")?;
        let cell_id = if let CellInfo::Provisioned(p) = &cells[0] {
            p.cell_id.clone()
        } else {
            return Err("wrong cell type found in happ".to_string());
        };

        let mut app_interfaces = admin_ws
            .list_app_interfaces()
            .await
            .map_err(|err| format!("could not list app interfaces: {:?}", err))?;
        let app_port = if app_interfaces.len() == 0 {
            let app_port = admin_ws
                .attach_app_interface(0)
                .await
                .map_err(|err| format!("error attaching app interface: {:?}", err))?;
            println!("attached new app port {app_port}");
            app_port
        } else {
            let app_port = app_interfaces.pop().unwrap();
            println!("found existing app port {app_port}");
            app_port
        };
        let app_ws = AppWebsocket::connect(format!("ws://127.0.0.1:{app_port}"))
            .await
            .map_err(|err| format!("error connecting app websocket: {:?}", err))?;
        println!("app websocket connected");

        let happ = Happ {
            conductor,
            admin_ws: Arc::new(admin_ws),
            app_ws: Arc::new(app_ws),
            cell_id: Arc::new(cell_id),
        };
        Ok(happ)
    }

    pub async fn create_profile(&self, nickname: String) -> Result<Profile, String> {
        let profile = Profile {
            nickname,
            fields: BTreeMap::new(),
        };
        let profile_record: Record = self
            .call_zome(
                self.cell_id.agent_pubkey().clone(),
                "profiles".into(),
                "create_profile".into(),
                profile.clone(),
            )
            .await?;
        println!("profile created {:#?}", profile_record);
        Ok(profile)
    }

    pub async fn fetch_profile(&self, agent_key: AgentPubKey) -> Result<Profile, String> {
        let result: Option<Record> = self
            .call_zome(
                self.cell_id.agent_pubkey().clone(),
                "profiles".into(),
                "get_agent_profile".into(),
                agent_key,
            )
            .await?;
        println!("record {result:?}");

        if let Some(record) = result {
            let profile = Profile::try_from(record)?;
            Ok(profile)
        } else {
            Err("no profile found".to_string())
        }
    }

    async fn call_zome<T, P>(
        &self,
        provenance: AgentPubKey,
        zome_name: ZomeName,
        fn_name: FunctionName,
        payload: P,
    ) -> Result<T, String>
    where
        T: std::fmt::Debug + DeserializeOwned,
        P: Serialize + std::fmt::Debug,
    {
        let (nonce, expires_at) = fresh_nonce(Timestamp::now())
            .map_err(|err| format!("error creating a nonce: {:?}", err))?;
        let payload = ExternIO::encode(payload)
            .map_err(|err| format!("error encoding payload: {:?}", err))?;
        let profile_zome_call_unsigned = ZomeCallUnsigned {
            cell_id: (*self.cell_id).clone(),
            zome_name,
            fn_name,
            payload,
            provenance,
            nonce,
            expires_at,
            cap_secret: None,
        };
        let signed_zome_call = ZomeCall::try_from_unsigned_zome_call(
            self.conductor.keystore(),
            profile_zome_call_unsigned,
        )
        .await
        .map_err(|err| format!("error signing zome call: {:?}", err))?;
        (*self.app_ws)
            .clone()
            .call_zome(signed_zome_call)
            .await
            .map_err(|err| format!("error calling zome: {:?}", err))?
            .decode::<T>()
            .map_err(|err| format!("error deserializing zome call result: {:?}", err))
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

mod tests {
    use crate::happ::Happ;
    use std::env::temp_dir;

    #[tokio::test(flavor = "multi_thread")]
    async fn holochain_start() {
        let mut path = temp_dir().to_path_buf();
        path.push("some_dir");
        println!("path {:?}", path);
        let happ = Happ::start_holochain_app(path).await.unwrap();
        (*happ.app_ws)
            .clone()
            .app_info("some_id".to_string())
            .await
            .unwrap();
    }

    // #[tokio::test(flavor = "multi_thread")]
    // async fn holochain_restartable() {
    //     let mut path = temp_dir().to_path_buf();
    //     path.push("some_dir");
    //     println!("path {:?}", path);
    //     {
    //         let mut happ = Happ::start_holochain_app(path).await.unwrap();
    //         let installed_apps = (*happ.admin_ws.clone()).list_apps(None).await.unwrap();
    //         println!("instlaled {installed_apps:?}");
    //         // .await
    //         // .unwrap();
    //     }

    //     // let happ = Happ::start_holochain_app(path).await.unwrap();
    //     // (*happ.app_ws)
    //     //     .clone()
    //     //     .app_info("some_id".to_string())
    //     //     .await
    //     //     .unwrap();
    // }
}
