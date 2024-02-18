use holochain::sweettest::SweetConductor;
use holochain::sweettest::{SweetConductorConfig, SweetLocalRendezvous};

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn messages() {
    use hdk::prelude::ActionHash;
    use holochain::sweettest::{SweetAgents, SweetDnaFile};
    use holomess_integrity::HoloMess;
    use std::path::Path;

    let mut conductor = SweetConductor::from_config_rendezvous(
        SweetConductorConfig::rendezvous(true),
        SweetLocalRendezvous::new().await,
    )
    .await;
    let dna_file_path = Path::new("../../../workdir/holomess.dna");
    let dna_file = SweetDnaFile::from_bundle(dna_file_path).await.unwrap();
    let zome_name = "holomess";
    let agents = SweetAgents::get(conductor.keystore(), 2).await;
    let apps = conductor
        .setup_app_for_agents("", &agents, [&dna_file])
        .await
        .unwrap();

    // check messages for one agent
    let zome = apps[0].cells()[0].zome(zome_name);
    let messages: Vec<HoloMess> = conductor.call(&zome, "get_messages", ()).await;
    assert_eq!(messages.len(), 0);

    let text = "test";
    let _action_hash: ActionHash = conductor.call(&zome, "create_message", text).await;

    let messages: Vec<HoloMess> = conductor.call(&zome, "get_messages", ()).await;
    assert_eq!(
        messages,
        vec![HoloMess {
            text: text.to_string()
        }]
    );

    // check messages for two agents

    // 2nd agent should see message of first agent
    let zome = apps[1].cells()[0].zome(zome_name);
    let messages: Vec<HoloMess> = conductor.call(&zome, "get_messages", ()).await;
    assert_eq!(
        messages,
        vec![HoloMess {
            text: text.to_string()
        }]
    );

    // 2nd agent creates message and should see two messages
    let _action_hash: ActionHash = conductor.call(&zome, "create_message", text).await;
    let messages: Vec<HoloMess> = conductor.call(&zome, "get_messages", ()).await;
    assert_eq!(
        messages,
        vec![
            HoloMess {
                text: text.to_string()
            },
            HoloMess {
                text: text.to_string()
            }
        ]
    );
}
