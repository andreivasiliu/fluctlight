use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::matrix_types::{Id, User, Device};

pub(crate) trait MatrixRequest {
    type Response;
}

pub(crate) mod get_federation_v1_version {
    use super::*;

    #[derive(Serialize, Deserialize)]
    pub(crate) struct RequestBody<'a> {
        #[serde(skip)]
        phantom: std::marker::PhantomData<&'a str>,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct Response<'a> {
        #[serde(borrow)]
        pub server: Server<'a>,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct Server<'a> {
        pub name: &'a str,
        pub version: &'a str,
    }

    impl<'a> MatrixRequest for RequestBody<'a> {
        type Response = Response<'a>;
    }
}

pub(crate) mod post_federation_v1_user_keys_query {
    use super::*;

    #[derive(Serialize, Deserialize)]
    pub(crate) struct RequestBody<'a> {
        #[serde(borrow)]
        pub device_keys: BTreeMap<&'a Id<User>, Vec<&'a Id<Device>>>,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct Response<'a> {
        #[serde(borrow)]
        pub device_keys: BTreeMap<&'a Id<User>, BTreeMap<&'a Id<Device>, DeviceKeys<'a>>>,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct DeviceKeys<'a> {
        #[serde(borrow)]
        pub user_id: &'a Id<User>,
        #[serde(borrow)]
        pub device_id: &'a Id<Device>,
        #[serde(borrow)]
        pub algorithms: Vec<EventEncryptionAlgorithm<'a>>,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) enum EventEncryptionAlgorithm<'a> {
        OlmV1Curve25519AesSha2,
        MegolmV1AesSha2,
        Other(&'a str),
    }

    impl<'a> MatrixRequest for RequestBody<'a> {
        type Response = Response<'a>;
    }
}
