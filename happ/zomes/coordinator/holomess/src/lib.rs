use hdk::prelude::*;
use holomess_integrity::{EntryTypes, HoloMess, LinkTypes};

#[hdk_extern]
pub fn get_messages(_: ()) -> ExternResult<Vec<HoloMess>> {
    let agent_key = agent_info()?.agent_latest_pubkey;
    let links = get_links(agent_key, LinkTypes::Message, None)?;
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
    let agent_key = agent_info()?.agent_latest_pubkey;
    let _link_hash = create_link(agent_key, action_hash.clone(), LinkTypes::Message, ())?;
    Ok(action_hash)
}
