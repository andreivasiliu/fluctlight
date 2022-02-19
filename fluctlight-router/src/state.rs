use std::{
    collections::BTreeMap,
    time::{Duration, SystemTime},
};

use ed25519_compact::KeyPair;
use serde::{Deserialize, Serialize};

use crate::{
    matrix_types::{Id, Key, ServerName},
    server_keys::ServerKeys,
};

pub(crate) struct State {
    // pub users: BTreeMap<Box<Id<User>>, UserState>,
    pub server_keys: BTreeMap<Box<Id<Key>>, ServerKey>,
    pub server_name: Box<Id<ServerName>>,
    pub foreign_server_keys: BTreeMap<Box<Id<ServerName>>, ServerKeys>,
}

// pub(crate) struct UserState {
//     pub(crate) name: Box<Id<User>>,
//     pub(crate) devices: Vec<Box<Id<Device>>>,
//     pub keys: Vec<(Box<Id<Device>>, ())>,
// }

pub(crate) struct ServerKey {
    pub public_key_base64: String,
    pub key_pair: KeyPair,
    pub valid_until: TimeStamp,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
#[serde(transparent)]
pub(crate) struct TimeStamp(u64);

impl TimeStamp {
    pub fn one_week_from_now() -> Self {
        let now = SystemTime::now();
        TimeStamp(
            now.checked_add(Duration::from_secs(60 * 60 * 24 * 7))
                .expect("Should always be in range")
                .duration_since(std::time::UNIX_EPOCH)
                .expect("Should always be positive")
                .as_secs(),
        )
    }

    pub fn as_secs(&self) -> u64 {
        self.0
    }
}

impl State {
    pub(crate) fn new() -> Self {
        let key_pair = KeyPair::generate();

        let public_key_base64 = base64::encode_config(&*key_pair.pk, base64::STANDARD_NO_PAD);

        let server_key = ServerKey {
            public_key_base64,
            key_pair,
            valid_until: TimeStamp::one_week_from_now(),
        };

        let mut server_keys = BTreeMap::new();
        server_keys.insert(
            Id::<Key>::try_boxed_from_str("ed25519:mykey").expect("Valid name"),
            server_key,
        );

        let foreign_server_keys = BTreeMap::new();

        State {
            // users: BTreeMap::new(),
            server_keys,
            server_name: Id::try_boxed_from_str("fluctlight-dev.demi.ro").unwrap(),
            foreign_server_keys,
        }
    }
}
