use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    path::Path,
    sync::Arc,
};

use get_port::Ops;
use hc_zome_profiles_integrity::Profile;
use hdk::prelude::{CellId, ExternIO, FunctionName, Record, Serialize, Timestamp, ZomeName};
use holochain::{
    conductor::{
        api::CellInfo,
        config::{AdminInterfaceConfig, ConductorConfig, KeystoreConfig},
        interface::InterfaceDriver,
        paths::DatabaseRootPath,
        Conductor, ConductorBuilder,
    },
    prelude::{
        kitsune_p2p::dependencies::{
            kitsune_p2p_types::config::{KitsuneP2pConfig, TransportConfig},
            url2::url2,
        },
        AppBundleSource, ZomeCallUnsigned,
    },
};
use holochain_client::{AdminWebsocket, AgentPubKey, AppWebsocket, InstallAppPayload, ZomeCall};
use holochain_nonce::fresh_nonce;
use lair_keystore_api::dependencies::serde::de::DeserializeOwned;

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
    pub async fn start_holochain_app() -> Result<Happ, String> {
        let temp_dir = tempfile::tempdir().map_err(|err| err.to_string())?;
        let mut conductor_config = ConductorConfig::default();
        conductor_config.environment_path =
            DatabaseRootPath::from(temp_dir.path().join("conductor/"));
        conductor_config.keystore = KeystoreConfig::LairServerInProc {
            lair_root: Some(temp_dir.path().join("lair/")),
        };
        let admin_port = get_port::tcp::TcpPort::in_range(
            "127.0.0.1",
            get_port::Range {
                min: 55000,
                max: 56000,
            },
        )
        .ok_or_else(|| {
            eprintln!("couldn't get a free port for admin interface");
            "couldn't get a free port for admin interface".to_string()
        })?;
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

        let conductor = ConductorBuilder::new()
            .config(conductor_config)
            .passphrase(Some(vec_to_locked("pass".as_bytes().to_owned()).unwrap()))
            .build()
            .await
            .map_err(|err| format!("building conductor failed {:#?}", err))?;
        println!("conductor built: config {:?}", conductor.config);

        let admin_port = conductor
            .get_arbitrary_admin_websocket_port()
            .ok_or_else(|| "could not get admin port".to_string())?;
        let mut admin_ws = AdminWebsocket::connect(format!("ws://127.0.0.1:{}", admin_port))
            .await
            .map_err(|err| format!("failed to connect admin web socket {:?}", err))?;
        println!("admin web soccket connected");

        let agent_key = admin_ws
            .generate_agent_pub_key()
            .await
            .map_err(|err| format!("error generating an agent pub key: {:?}", err))?;
        println!("generated agent pub key {:?}", agent_key);

        let install_app_payload = InstallAppPayload {
            source: AppBundleSource::Path(Path::new("happ/workdir/holomess.happ").to_path_buf()),
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

        let cells = app_info
            .cell_info
            .get("holomess")
            .ok_or_else(|| "cell not found")?;
        let cell_id = if let CellInfo::Provisioned(p) = &cells[0] {
            p.cell_id.clone()
        } else {
            return Err("wrong cell type found in happ".to_string());
        };

        let app_port = admin_ws
            .attach_app_interface(0)
            .await
            .map_err(|err| format!("error attaching app interface: {:?}", err))?;
        println!("app port is {app_port}");
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

    pub async fn create_profile(&self) -> Result<(), String> {
        let profile = Profile {
            nickname: "peter".to_string(),
            fields: BTreeMap::new(),
        };
        let profile: Record = self
            .call_zome(
                self.cell_id.agent_pubkey().clone(),
                "profiles".into(),
                "create_profile".into(),
                profile,
            )
            .await?;
        println!("profile created {:#?}", profile);
        Ok(())
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
