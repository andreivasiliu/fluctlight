use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    matrix_types::{Id, Key, ServerName},
    state::TimeStamp,
};

#[derive(Serialize, Deserialize)]
pub(crate) struct ServerKeys {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_verify_keys: Option<BTreeMap<Box<Id<Key>>, OldVerifyKey>>,
    pub server_name: Box<Id<ServerName>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signatures: Option<Signatures>,
    pub valid_until_ts: Option<TimeStamp>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub verify_keys: BTreeMap<Box<Id<Key>>, VerifyKey>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct OldVerifyKey {
    pub expired_ts: TimeStamp,
    pub key: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct VerifyKey {
    pub key: String,
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Signatures {
    pub signatures: BTreeMap<Box<Id<ServerName>>, BTreeMap<Box<Id<Key>>, String>>,
}
