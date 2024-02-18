use hdk::prelude::*;
use holomess_integrity::{EntryTypes, HoloMess, LinkTypes};

const ALL_MESSES_BASE: &str = "all_messes";

#[hdk_extern]
pub fn get_messages(_: ()) -> ExternResult<Vec<HoloMess>> {
    let path = Path::from(ALL_MESSES_BASE);
    let links = get_links(path.path_entry_hash()?, LinkTypes::Message, None)?;
    let get_inputs = links
        .into_iter()
        .map(|link| {
            GetInput::new(
                HoloHash::try_from(link.target).expect("must be a valid link hash"),
                GetOptions::default(),
            )
        })
        .collect();
    let records = HDK.with(|hdk| hdk.borrow().get(get_inputs))?;
    let messages = records
        .into_iter()
        .flatten()
        .map(|record| HoloMess::try_from(record))
        .flatten()
        .collect();
    Ok(messages)
}

#[hdk_extern]
pub fn create_message(message: String) -> ExternResult<ActionHash> {
    let holo_mess = HoloMess { text: message };
    let action_hash = create_entry(EntryTypes::Message(holo_mess))?;
    // link to agent key base
    let agent_key = agent_info()?.agent_latest_pubkey;
    let _agent_link_hash = create_link(agent_key, action_hash.clone(), LinkTypes::Message, ())?;
    // link to all messages base
    let path = Path::from(ALL_MESSES_BASE);
    let _all_link_hash = create_link(
        path.path_entry_hash()?,
        action_hash.clone(),
        LinkTypes::Message,
        (),
    )?;

    Ok(action_hash)
}
