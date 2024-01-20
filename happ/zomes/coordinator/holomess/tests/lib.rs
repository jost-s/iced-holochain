use holochain::sweettest::SweetConductor;
use holochain::sweettest::{SweetConductorConfig, SweetLocalRendezvous};

#[cfg(test)]
#[tokio::test(flavor = "multi_thread")]
async fn messages() {
    use hdk::prelude::ActionHash;
    use holochain::sweettest::SweetDnaFile;
    use holomess_integrity::HoloMess;
    use std::path::Path;

    let mut conductor = SweetConductor::from_config_rendezvous(
        SweetConductorConfig::rendezvous(),
        SweetLocalRendezvous::new().await,
    )
    .await;
    let dna_file_path = Path::new("../../../workdir/holomess.dna");
    let dna_file = SweetDnaFile::from_bundle(dna_file_path).await.unwrap();
    let app = conductor.setup_app("", [&dna_file]).await.unwrap();
    let zome = app.cells()[0].zome("holomess");

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
}
