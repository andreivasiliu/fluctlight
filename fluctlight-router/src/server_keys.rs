use std::collections::BTreeMap;

use ed25519_compact::{PublicKey, SecretKey, Signature};
use serde::{Deserialize, Serialize};
use sha2::Digest;

use crate::{
    matrix_types::{Event, Id, Key, ServerName},
    state::{ServerKeyPair, State, TimeStamp},
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

#[derive(Serialize, Deserialize, Default, Debug)]
#[serde(transparent)]
pub(crate) struct Signatures {
    pub signatures: BTreeMap<Box<Id<ServerName>>, BTreeMap<Box<Id<Key>>, String>>,
}

// FIXME: Switch to signable
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

pub(crate) trait Signable: Serialize {
    fn signatures_mut(&mut self) -> &mut Option<Signatures>;

    fn take_event_id(&mut self) -> Option<Box<Id<Event>>>;
    fn put_event_id(&mut self, event_id: Option<Box<Id<Event>>>);

    fn sign(&mut self, state: &State) {
        let mut signatures = self.signatures_mut().take().unwrap_or_default();
        let event_id = self.take_event_id();
        let mut server_signatures = BTreeMap::new();

        let bytes = serde_json::to_vec(&self).expect("Serialization should always succeed");
        eprintln!(
            "Signing signable: \n---\n{}\n---\n",
            String::from_utf8_lossy(&bytes)
        );

        for (key_name, server_key) in &state.server_key_pairs {
            let noise = None;
            let signature = server_key.key_pair.sk.sign(&bytes, noise);
            let sig_b64 = base64::encode_config(&*signature, base64::STANDARD_NO_PAD);

            server_signatures.insert(key_name.clone(), sig_b64);
        }

        signatures
            .signatures
            .insert(state.server_name.to_owned(), server_signatures);

        *self.signatures_mut() = Some(signatures);
        self.put_event_id(event_id);
    }

    fn verify(&mut self, state: &State, server_name: &Id<ServerName>) -> bool {
        let signatures = self.signatures_mut().take().unwrap_or_default();

        let bytes = serde_json::to_vec(self).expect("Serialization should always succeed");

        eprintln!(
            "Verifying signable: \n---\n{}\n---\n",
            String::from_utf8_lossy(&bytes)
        );

        for (signing_server_name, server_signatures) in &signatures.signatures {
            if server_name == &**signing_server_name {
                let server_public_keys = match state.foreign_server_keys.get(server_name) {
                    Some(value) => value,
                    None => continue,
                };

                for (key_name, signature) in server_signatures {
                    let public_key = match server_public_keys.verify_keys.get(key_name) {
                        Some(value) => value,
                        None => continue,
                    };
                    let public_key_bytes =
                        base64::decode_config(&public_key.key, base64::STANDARD_NO_PAD)
                            .expect("Key already validated");
                    let verify_key =
                        PublicKey::from_slice(&public_key_bytes).expect("Key already validated");

                    let signature_bytes = base64::decode_config(signature, base64::STANDARD_NO_PAD)
                        .expect("Signature already validated");
                    let signature = Signature::from_slice(&signature_bytes)
                        .expect("Signature already validated");

                    match verify_key.verify(&bytes, &signature) {
                        Ok(()) => {
                            eprintln!("Key check for {} succeeded", key_name);
                            *self.signatures_mut() = Some(signatures);
                            return true;
                        }
                        Err(err) => {
                            eprintln!("Key check for {} failed: {}", key_name, err);
                        }
                    }
                }
            }
        }

        *self.signatures_mut() = Some(signatures);

        false
    }
}

pub(crate) trait Hashable: Signable + Serialize {
    fn event_id_mut(&mut self) -> &mut Option<Box<Id<Event>>>;
    fn hashes_mut(&mut self) -> &mut Option<BTreeMap<String, String>>;

    fn hash(&mut self) {
        let signatures = self.signatures_mut().take();
        self.hashes_mut().take();
        self.event_id_mut().take();

        let mut hashes = BTreeMap::new();

        let bytes = serde_json::to_vec(&self).expect("Serialization should always succeed");

        let mut hasher = sha2::Sha256::new();
        hasher.update(bytes);
        let sha256_hash = hasher.finalize();

        let b64_sha256_hash =
            base64::encode_config(sha256_hash.as_slice(), base64::STANDARD_NO_PAD);

        hashes.insert("sha256".to_string(), b64_sha256_hash);

        *self.hashes_mut() = Some(hashes);

        let bytes = serde_json::to_vec(&self).expect("Serialization should always succeed");

        eprintln!(
            "Hashing hashable: \n---\n{}\n---\n",
            String::from_utf8_lossy(&bytes)
        );

        let mut hasher = sha2::Sha256::new();
        hasher.update(bytes);
        let sha256_hash = hasher.finalize();

        let event_id = format!(
            "${}",
            base64::encode_config(sha256_hash.as_slice(), base64::URL_SAFE_NO_PAD)
        );
        let event_id = Id::<Event>::try_boxed_from_str(&event_id).expect("Valid event ID");

        *self.signatures_mut() = signatures;
        *self.event_id_mut() = Some(event_id);
    }
}
