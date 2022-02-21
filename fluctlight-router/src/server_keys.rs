use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    matrix_types::{Id, Key, ServerName},
    state::{ServerKeyPair, TimeStamp},
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

#[derive(Serialize, Deserialize, Default)]
#[serde(transparent)]
pub(crate) struct Signatures {
    pub signatures: BTreeMap<Box<Id<ServerName>>, BTreeMap<Box<Id<Key>>, String>>,
}

impl ServerKeys {
    pub(crate) fn sign(
        &mut self,
        server_name: &Id<ServerName>,
        server_key_pairs: &BTreeMap<Box<Id<Key>>, ServerKeyPair>,
    ) {
        let mut signatures = self.signatures.take().unwrap_or_default();
        let mut server_signatures = BTreeMap::new();

        let response_bytes =
            serde_json::to_vec(&self).expect("Serialization should always succeed");

        for (key_name, server_key) in server_key_pairs {
            let noise = None;
            let signature = server_key.key_pair.sk.sign(&response_bytes, noise);
            let sig_b64 = base64::encode_config(&*signature, base64::STANDARD_NO_PAD);

            server_signatures.insert(key_name.clone(), sig_b64);
        }

        signatures
            .signatures
            .insert(server_name.to_owned(), server_signatures);

        self.signatures = Some(signatures);
    }
}
