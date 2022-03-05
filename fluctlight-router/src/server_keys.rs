use std::collections::BTreeMap;

use ed25519_compact::{PublicKey, Signature};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use smallvec::SmallVec;

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

// Used to sign server keys, PDUs, and outgoing federation requests.
pub(crate) trait Signable: Serialize {
    fn signatures_mut(&mut self) -> &mut Option<Signatures>;
    fn signatures(&self) -> &Option<Signatures>;

    fn take_event_id(&mut self) -> Option<Box<Id<Event>>>;
    fn put_event_id(&mut self, event_id: Option<Box<Id<Event>>>);

    fn sign(&mut self, state: &State) {
        let mut signatures = self.signatures_mut().take().unwrap_or_default();
        // FIXME: take unsigned
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

    fn verify(&self, state: &State, server_name: &Id<ServerName>) -> Result<(), &'static str> {
        let signatures = match self.signatures() {
            Some(signatures) => signatures,
            None => return Err("The object has no signatures"),
        };

        let bytes = serde_json::to_vec(self).expect("Serialization should always succeed");

        // eprintln!(
        //     "Verifying signable: \n---\n{}\n---\n",
        //     String::from_utf8_lossy(&bytes)
        // );

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
                            return Ok(());
                        }
                        Err(err) => {
                            // TODO: Figure out if this is just a warning or if
                            // the check needs to abort here
                            eprintln!("Key check for {} failed: {}", key_name, err);
                        }
                    }
                }
            }
        }

        Err("No keys succeeded")
    }
}

pub(crate) trait Hashable: Signable + Serialize {
    fn event_id_mut(&mut self) -> &mut Option<Box<Id<Event>>>;
    fn hashes_mut(&mut self) -> &mut Option<BTreeMap<String, String>>;

    fn hash(&mut self) {
        let mut scratch_buffer = SmallVec::<[u8; 64]>::new();
        scratch_buffer.resize(64, 0);

        let signatures = self.signatures_mut().take();
        let mut hashes = self.hashes_mut().take().unwrap_or_default();
        self.event_id_mut().take();

        let mut hasher = sha2::Sha256::new();
        serde_json::to_writer(&mut hasher, &self).expect("Serialization should always succeed");
        let sha256_hash = hasher.finalize();

        let hash_size = base64::encode_config_slice(
            sha256_hash.as_slice(),
            base64::STANDARD_NO_PAD,
            &mut scratch_buffer[..],
        );
        let b64_sha256_hash: &str =
            std::str::from_utf8(&scratch_buffer[..hash_size]).expect("Base64 is always a string");

        if let Some(existing_hash) = hashes.get("sha256") {
            if existing_hash != b64_sha256_hash {
                // FIXME
                // eprintln!("Hashes do not match: {}", b64_sha256_hash);
            }
        } else {
            hashes.insert("sha256".to_string(), b64_sha256_hash.to_string());
        }

        *self.hashes_mut() = Some(hashes);

        let mut hasher = sha2::Sha256::new();
        serde_json::to_writer(&mut hasher, &self).expect("Serialization should always succeed");
        let sha256_hash = hasher.finalize();

        // eprintln!(
        //     "Hashing hashable: \n---\n{}\n---\n",
        //     String::from_utf8_lossy(&bytes)
        // );

        scratch_buffer[0] = b'$';
        let hash_size = base64::encode_config_slice(
            sha256_hash.as_slice(),
            base64::URL_SAFE_NO_PAD,
            &mut scratch_buffer[1..64],
        );
        let b64_sha256_hash: &str = std::str::from_utf8(&scratch_buffer[..hash_size + 1])
            .expect("Base64 is always a string");

        let event_id = Id::<Event>::try_boxed_from_str(b64_sha256_hash).expect("Valid event ID");

        *self.signatures_mut() = signatures;
        *self.event_id_mut() = Some(event_id);
    }

    fn generate_event_id(&mut self) {
        self.event_id_mut().take();
        let signatures = self.signatures_mut().take();

        let mut hasher = sha2::Sha256::new();
        serde_json::to_writer(&mut hasher, &self).expect("Serialization should always succeed");
        let sha256_hash = hasher.finalize();

        // eprintln!(
        //     "Hashing hashable: \n---\n{}\n---\n",
        //     String::from_utf8_lossy(&bytes)
        // );

        let mut scratch_buffer = SmallVec::<[u8; 64]>::new();
        scratch_buffer.resize(64, 0);
        scratch_buffer[0] = b'$';
        let hash_size = base64::encode_config_slice(
            sha256_hash.as_slice(),
            base64::URL_SAFE_NO_PAD,
            &mut scratch_buffer[1..64],
        );
        let b64_sha256_hash: &str = std::str::from_utf8(&scratch_buffer[..hash_size + 1])
            .expect("Base64 is always a string");

        let event_id = Id::<Event>::try_boxed_from_str(b64_sha256_hash).expect("Valid event ID");

        *self.event_id_mut() = Some(event_id);
        *self.signatures_mut() = signatures;
    }
}
