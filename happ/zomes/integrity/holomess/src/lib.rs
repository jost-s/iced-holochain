use hdi::prelude::*;

#[hdk_entry_helper]
#[derive(Clone, PartialEq, PartialOrd)]
pub struct HoloMess {
    pub text: String,
}

#[hdk_entry_defs]
#[unit_enum(UnitEntryTypes)]
pub enum EntryTypes {
    Message(HoloMess),
}

#[hdk_link_types]
pub enum LinkTypes {
    Message,
}
