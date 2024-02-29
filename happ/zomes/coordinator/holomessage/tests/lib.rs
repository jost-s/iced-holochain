use hdk::prelude::Record;
use holochain::sweettest::SweetConductor;
use holochain::sweettest::{SweetConductorConfig, SweetLocalRendezvous};

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn messages() {
    use hdk::prelude::ActionHash;
    use holochain::sweettest::{SweetAgents, SweetDnaFile};
    use holomessage_integrity::HoloMessage;
    use std::path::Path;

    let mut conductor = SweetConductor::from_config_rendezvous(
        SweetConductorConfig::rendezvous(true),
        SweetLocalRendezvous::new().await,
    )
    .await;
    let dna_file_path = Path::new("../../../workdir/holomessage.dna");
    let dna_file = SweetDnaFile::from_bundle(dna_file_path).await.unwrap();
    let zome_name = "holomessage";
    let agents = SweetAgents::get(conductor.keystore(), 2).await;
    let apps = conductor
        .setup_app_for_agents("", &agents, [&dna_file])
        .await
        .unwrap();

    // check messages for one agent
    let zome = apps[0].cells()[0].zome(zome_name);
    println!("does this fail?");
    let messages: Vec<Record> = conductor.call(&zome, "get_messages", ()).await;
    assert_eq!(messages.len(), 0);

    let message_1 = "text_1";
    let _action_hash: ActionHash = conductor.call(&zome, "create_message", message_1).await;

    println!("does this fail 2?");
    let messages: Vec<Record> = conductor.call(&zome, "get_messages", ()).await;
    let messages: Vec<HoloMessage> = messages
        .into_iter()
        .map(TryFrom::try_from)
        .flatten()
        .collect();
    assert_eq!(
        messages,
        vec![HoloMessage {
            text: message_1.to_string()
        }]
    );

    // check messages for two agents

    // 2nd agent should see message of first agent
    let zome = apps[1].cells()[0].zome(zome_name);
    let messages: Vec<Record> = conductor.call(&zome, "get_messages", ()).await;
    let messages: Vec<HoloMessage> = messages
        .into_iter()
        .map(TryFrom::try_from)
        .flatten()
        .collect();
    assert_eq!(
        messages,
        vec![HoloMessage {
            text: message_1.to_string()
        }]
    );

    // 2nd agent creates message and should see two messages
    // ordered by timestamp
    let message_2 = "text_2";
    let _action_hash: ActionHash = conductor.call(&zome, "create_message", message_2).await;

    let messages: Vec<Record> = conductor.call(&zome, "get_messages", ()).await;
    let messages: Vec<HoloMessage> = messages
        .into_iter()
        .map(TryFrom::try_from)
        .flatten()
        .collect();
    assert_eq!(
        messages,
        vec![
            HoloMessage {
                text: message_2.to_string()
            },
            HoloMessage {
                text: message_1.to_string()
            }
        ]
    );
}
