use hdk::prelude::*;
use holomessage_integrity::{EntryTypes, HoloMessage, LinkTypes};

const ALL_MESSAGES_BASE: &str = "all_messages";

#[hdk_extern]
pub fn get_messages(_: ()) -> ExternResult<Vec<Record>> {
    let path = Path::from(ALL_MESSAGES_BASE);
    let links = get_links(path.path_entry_hash()?, LinkTypes::HoloMessage, None)?;
    let get_inputs = links
        .into_iter()
        .map(|link| {
            GetInput::new(
                HoloHash::try_from(link.target).expect("must be a valid link hash"),
                GetOptions::default(),
            )
        })
        .collect();
    let mut records: Vec<Record> = HDK
        .with(|hdk| hdk.borrow().get(get_inputs))?
        .into_iter()
        .flatten()
        .collect();
    records.sort_by(|a, b| b.action().timestamp().cmp(&a.action().timestamp()));
    Ok(records)
}

#[hdk_extern]
pub fn create_message(message: String) -> ExternResult<ActionHash> {
    let holo_message = HoloMessage { text: message };
    let action_hash = create_entry(EntryTypes::HoloMessage(holo_message))?;
    // link to agent key base
    let agent_key = agent_info()?.agent_latest_pubkey;
    let _agent_link_hash = create_link(agent_key, action_hash.clone(), LinkTypes::HoloMessage, ())?;
    // link to all messages base
    let path = Path::from(ALL_MESSAGES_BASE);
    let _all_link_hash = create_link(
        path.path_entry_hash()?,
        action_hash.clone(),
        LinkTypes::HoloMessage,
        (),
    )?;

    Ok(action_hash)
}
