use std::{
    collections::BTreeMap,
    io::Write,
    sync::{RwLock, RwLockReadGuard},
    time::SystemTime,
};

use ed25519_compact::{KeyPair, PublicKey};
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::{
    interner::{ArcStr, Interner},
    matrix_types::{Event, Id, Key, Room, ServerName},
    persistence::RoomPersistence,
    playground::ParsedPDU,
    rendered_json::RenderedJson,
    server_keys::{ServerKeys, VerifyKey},
};

pub(crate) struct State {
    // pub users: BTreeMap<Box<Id<User>>, UserState>,
    pub server_key_pairs: BTreeMap<Box<Id<Key>>, ServerKeyPair>,
    pub server_name: Box<Id<ServerName>>,
    pub foreign_server_keys_json:
        BTreeMap<Box<Id<ServerName>>, Vec<RenderedJson<'static, ServerKeys>>>,
    pub foreign_key_cache: BTreeMap<(Box<Id<ServerName>>, Box<Id<Key>>), PublicKey>,
    persistent: RwLock<Persistent>,
    ephemeral: RwLock<Ephemeral>,
}

// TODO: Just a quick and dirty persistence store; needs to be fundamentally different
#[derive(Serialize, Deserialize)]
pub(crate) struct Persistent {
    pub rooms: BTreeMap<Box<Id<Room>>, RoomState>,
    pub net_log_index: usize,
}

// TODO: Same as above, but even quicker, and even dirtier
pub(crate) struct Ephemeral {
    pub own_server_keys: ServerKeys,
    pub rendered_server_keys: Box<RawValue>,
    pub rooms: BTreeMap<Box<Id<Room>>, EphemeralRoomState>,
}

// pub(crate) struct UserState {
//     pub(crate) name: Box<Id<User>>,
//     pub(crate) devices: Vec<Box<Id<Device>>>,
//     pub keys: Vec<(Box<Id<Device>>, ())>,
// }

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct RoomState {
    pub pdu_blobs: Vec<String>,
    pub room_db: Option<String>,
}

#[derive(Default)]
pub(crate) struct EphemeralRoomState {
    pub pdus: BTreeMap<ArcStr<Id<Event>>, ParsedPDU>,
    pub pdus_by_timestamp: BTreeMap<TimeStamp, ArcStr<Id<Event>>>,
    pub interner: Interner,
    pub room_persistence: Option<RoomPersistence>,
}

#[derive(Clone)]
pub(crate) struct ServerKeyPair {
    pub public_key_base64: String,
    pub key_pair: KeyPair,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct ServerKeyPairBase64 {
    pub public_key_base64: String,
    pub key_pair_base64: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
pub(crate) struct TimeStamp(u128);

impl TimeStamp {
    pub fn one_week_from_today() -> Self {
        let today = TimeStamp::today();
        TimeStamp(today.0 + 1000 * 60 * 60 * 24 * 7)
    }

    pub fn six_days_from_today() -> Self {
        let today = TimeStamp::today();
        TimeStamp(today.0 + 1000 * 60 * 60 * 24 * 6)
    }

    pub fn today() -> Self {
        let now = SystemTime::now();
        let millis_now = now
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Should always be positive")
            .as_millis();

        let one_day = 1000 * 60 * 60 * 24;
        let millis_today = (millis_now / one_day) * one_day;

        TimeStamp(millis_today)
    }

    pub fn now() -> Self {
        let now = SystemTime::now();
        TimeStamp(
            now.duration_since(std::time::UNIX_EPOCH)
                .expect("Should always be positive")
                .as_millis(),
        )
    }

    pub fn as_millis(&self) -> u128 {
        self.0
    }
}

impl State {
    pub(crate) fn new() -> Self {
        let server_key_pairs = load_server_key_pairs();

        let mut verify_keys = BTreeMap::new();

        let server_name = Id::try_boxed_from_str("fluctlight-dev.demi.ro").unwrap();

        for (key_name, key_pair) in &server_key_pairs {
            let verify_key = VerifyKey {
                key: key_pair.public_key_base64.clone(),
            };

            verify_keys.insert(key_name.clone(), verify_key);
        }

        let valid_until_ts = TimeStamp::one_week_from_today();

        let mut own_server_keys = ServerKeys {
            old_verify_keys: None,
            server_name: server_name.clone(),
            signatures: None,
            valid_until_ts: Some(valid_until_ts),
            verify_keys,
        };
        own_server_keys.sign(&server_name, &server_key_pairs);

        let mut foreign_server_keys_json = BTreeMap::new();
        let mut foreign_server_keys = BTreeMap::new();
        let rendered_json = RenderedJson::from_trusted(
            serde_json::to_string(&own_server_keys).expect("Valid JSON"),
        );
        foreign_server_keys_json.insert(server_name.clone(), vec![rendered_json]);
        foreign_server_keys.insert(server_name, vec![own_server_keys.clone()]);

        foreign_server_keys.extend(load_foreign_keys());

        let mut foreign_key_cache = BTreeMap::new();

        for (server_name, server_keys_list) in &foreign_server_keys {
            for server_keys in server_keys_list {
                for (key_name, verify_key) in &server_keys.verify_keys {
                    let public_key = &verify_key.key;
                    let public_key_bytes =
                        base64::decode_config(public_key, base64::STANDARD_NO_PAD)
                            .expect("Key already validated");
                    let public_key =
                        PublicKey::from_slice(&public_key_bytes).expect("Key already validated");

                    let key_id = (server_name.to_owned(), key_name.to_owned());
                    foreign_key_cache.insert(key_id, public_key);
                    // FIXME: Check that it wasn't overwritten with something else
                }
            }
        }

        let rendered_server_keys: Box<RawValue> = serde_json::value::to_raw_value(&own_server_keys)
            .expect("Serialization should always succeed");

        let persistent = Persistent::load();
        let ephemeral = Ephemeral {
            rooms: BTreeMap::new(),
            own_server_keys,
            rendered_server_keys,
        };

        State {
            // users: BTreeMap::new(),
            server_key_pairs,
            server_name: Id::try_boxed_from_str("fluctlight-dev.demi.ro").unwrap(),
            foreign_server_keys_json,
            foreign_key_cache,
            persistent: RwLock::new(persistent),
            ephemeral: RwLock::new(ephemeral),
        }
    }

    pub fn persistent(&self) -> RwLockReadGuard<Persistent> {
        // TODO: corrupted; should use an older backup instead
        match self.persistent.read() {
            Ok(guard) => guard,
            Err(poison_error) => poison_error.into_inner(),
        }
    }

    pub fn with_persistent_mut<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut Persistent) -> R,
    {
        let mut persistent = self
            .persistent
            .write()
            .expect("Lock poisoned; cannot do more changes on corrupt data");

        // TODO: maybe reload from disk on panics and unpoison
        let result = f(&mut *persistent);

        persistent.save();

        result
    }

    pub fn ephemeral(&self) -> RwLockReadGuard<Ephemeral> {
        // TODO: corrupted; should use an older backup instead
        match self.ephemeral.read() {
            Ok(guard) => guard,
            Err(poison_error) => poison_error.into_inner(),
        }
    }

    pub fn with_ephemeral_mut<R, F>(&self, f: F) -> R
    where
        F: FnOnce(&mut Ephemeral) -> R,
    {
        let mut ephemeral = self
            .ephemeral
            .write()
            .expect("Lock poisoned; cannot do more changes on corrupt data");

        // Mimic the persistent interface to catch lifetime issues early
        f(&mut *ephemeral)
    }

    pub fn get_server_key(
        &self,
        server_name: &Id<ServerName>,
        key_name: &Id<Key>,
    ) -> Option<&PublicKey> {
        // FIXME: Timestamp should not be ignored
        self.foreign_key_cache
            .get(&(server_name.to_owned(), key_name.to_owned()))
    }
}

impl Persistent {
    fn load() -> Self {
        if !std::path::Path::new("persistent.json").exists() {
            eprintln!("Creating new persistent state...");
            return Persistent {
                rooms: BTreeMap::new(),
                net_log_index: 0,
            };
        }

        // FIXME: fix unwraps
        let persistent_file = std::fs::File::open("persistent.json").unwrap();
        serde_json::from_reader(persistent_file).unwrap()
    }

    fn save(&self) {
        // FIXME: fix unwraps
        let persistent_file = std::fs::File::create("persistent.json").unwrap();

        // FIXME: fix unwraps
        serde_json::to_writer_pretty(persistent_file, self).unwrap();
    }
}

fn save_server_key_pairs(key_pairs: BTreeMap<Box<Id<Key>>, ServerKeyPair>) {
    #[cfg(unix)]
    use std::os::unix::prelude::OpenOptionsExt;

    // FIXME: fix unwraps
    #[cfg(unix)]
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .mode(0o600)
        .open("server_keys.json.tmp")
        .unwrap();

    #[cfg(not(unix))]
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .open("server_keys.json.tmp")
        .unwrap();

    let key_pairs_base64: BTreeMap<Box<Id<Key>>, ServerKeyPairBase64> = key_pairs
        .into_iter()
        .map(|(key_name, key_pair)| {
            (
                key_name,
                ServerKeyPairBase64 {
                    public_key_base64: key_pair.public_key_base64,
                    key_pair_base64: base64::encode(&*key_pair.key_pair),
                },
            )
        })
        .collect();

    serde_json::to_writer_pretty(&mut file, &key_pairs_base64).unwrap();
    file.write(b"\n").unwrap();
    drop(file);
    std::fs::rename("server_keys.json.tmp", "server_keys.json").unwrap();
}

fn load_server_key_pairs() -> BTreeMap<Box<Id<Key>>, ServerKeyPair> {
    if !std::path::Path::new("server_keys.json").exists() {
        eprintln!("Generating new server keys...");
        let key_pairs = generate_server_key_pairs();
        save_server_key_pairs(key_pairs.clone());
        return key_pairs;
    }

    // FIXME: fix unwraps
    let key_file = std::fs::File::open("server_keys.json").unwrap();

    let key_pairs_base64: BTreeMap<Box<Id<Key>>, ServerKeyPairBase64> =
        serde_json::from_reader(key_file).unwrap();
    key_pairs_base64
        .into_iter()
        .map(|(key_name, key_pair)| {
            (
                key_name,
                ServerKeyPair {
                    public_key_base64: key_pair.public_key_base64,
                    // FIXME: fix unwraps
                    key_pair: KeyPair::from_slice(
                        &base64::decode(key_pair.key_pair_base64).unwrap(),
                    )
                    .unwrap(),
                },
            )
        })
        .collect()
}

fn generate_server_key_pairs() -> BTreeMap<Box<Id<Key>>, ServerKeyPair> {
    let key_pair = KeyPair::generate();

    let public_key_base64 = base64::encode_config(&*key_pair.pk, base64::STANDARD_NO_PAD);

    let name_suffix = public_key_base64
        .chars()
        .filter(|&c| c != '/' && c != '+')
        .take(6)
        .collect::<String>();
    let server_key_name = format!("ed25519:{}", name_suffix);

    let server_key_name =
        Id::<Key>::try_boxed_from_str(&server_key_name).expect("Key format is valid");

    let server_key_pair = ServerKeyPair {
        public_key_base64: public_key_base64.clone(),
        key_pair,
    };

    let mut server_key_pairs = BTreeMap::new();
    server_key_pairs.insert(server_key_name.clone(), server_key_pair);

    server_key_pairs
}

fn load_foreign_keys() -> BTreeMap<Box<Id<ServerName>>, Vec<ServerKeys>> {
    if !std::path::Path::new("foreign_keys.json").exists() {
        return BTreeMap::new();
    }

    // FIXME: fix unwraps
    let key_file = std::fs::File::open("foreign_keys.json").unwrap();
    serde_json::from_reader(key_file).unwrap()
}

impl State {
    pub(crate) fn render_own_server_keys(&self) -> Box<RawValue> {
        // FIXME: Use read-only rwlock when regeneration is not necesarry
        self.with_ephemeral_mut(|ephemeral_state| {
            let valid_until = ephemeral_state
                .own_server_keys
                .valid_until_ts
                .expect("Own keys should always have a timestamp");

            // Make it always be at least six days in the future
            if valid_until.0 <= TimeStamp::six_days_from_today().0 {
                eprintln!("Re-rendering server keys");
                ephemeral_state.own_server_keys.valid_until_ts =
                    Some(TimeStamp::one_week_from_today());
                ephemeral_state
                    .own_server_keys
                    .sign(&self.server_name, &self.server_key_pairs);
                ephemeral_state.rendered_server_keys =
                    serde_json::value::to_raw_value(&ephemeral_state.own_server_keys)
                        .expect("Serialization should always succeed");
            }

            ephemeral_state.rendered_server_keys.clone()
        })
    }
}
