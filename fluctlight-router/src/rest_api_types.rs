use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::{
    matrix_types::{Device, Event, Id, Key, Room, ServerName, User},
    state::TimeStamp,
};

pub(crate) struct GenericRequest<Path, QueryString, Body> {
    pub path: Path,
    pub query_string: QueryString,
    pub body: Body,
}

impl<'r, Path, QueryString, Body> GenericRequest<Path, QueryString, Body> {
    pub fn new(path: Path, query_string: QueryString, body: Body) -> Self {
        GenericRequest {
            path,
            query_string,
            body,
        }
    }
}

pub(crate) trait MatrixRequest {
    type Response;
    const PATH_SPEC: &'static str;
}

#[derive(Serialize, Deserialize)]
pub(crate) struct EmptyPath<'a> {
    #[serde(skip)]
    phantom: std::marker::PhantomData<&'a ()>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct EmptyBody {}

#[derive(Serialize, Deserialize)]
pub(crate) struct EmptyQS {}

pub(crate) mod get_federation_v1_version {
    use super::*;

    pub(crate) type Request<'a> = GenericRequest<RequestPath<'a>, EmptyQS, EmptyBody>;

    impl<'a> MatrixRequest for Request<'a> {
        type Response = Response<'a>;

        const PATH_SPEC: &'static str = "/_matrix/federation/:version/version";
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct RequestPath<'a> {
        #[serde(borrow)]
        pub version: &'a str,
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
}

pub(crate) mod get_key_v2_server {
    use super::*;

    pub(crate) type Request<'a> = GenericRequest<RequestPath<'a>, EmptyQS, EmptyBody>;

    impl<'a> MatrixRequest for Request<'a> {
        type Response = Response<'a>;

        const PATH_SPEC: &'static str = "/_matrix/key/:version/server/?key_id";
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct RequestPath<'a> {
        pub version: &'a str,
        #[serde(borrow)]
        pub key_id: Option<&'a Id<Key>>,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct Response<'a> {
        #[serde(borrow)]
        pub old_verify_keys: Option<BTreeMap<&'a Id<Key>, OldVerifyKey<'a>>>,
        pub server_name: &'a Id<ServerName>,
        pub signatures: Option<Signatures<'a>>,
        pub valid_until_ts: Option<TimeStamp>,
        pub verify_keys: Option<BTreeMap<&'a Id<Key>, VerifyKey<'a>>>,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct OldVerifyKey<'a> {
        pub expired_ts: TimeStamp,
        pub key: &'a str,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct VerifyKey<'a> {
        pub key: &'a str,
    }

    #[derive(Serialize, Deserialize)]
    #[serde(transparent)]
    pub(crate) struct Signatures<'a> {
        #[serde(borrow)]
        pub signatures: BTreeMap<&'a Id<ServerName>, BTreeMap<&'a Id<Key>, String>>,
    }
}

pub(crate) mod post_key_v2_query {
    use crate::rendered_json::RenderedJson;

    use super::*;

    pub(crate) type Request<'a> = GenericRequest<RequestPath<'a>, EmptyQS, RequestBody<'a>>;

    impl<'a> MatrixRequest for Request<'a> {
        type Response = Response<'a>;

        const PATH_SPEC: &'static str = "/_matrix/key/:version/query";
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct RequestPath<'a> {
        pub version: &'a str,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct RequestBody<'a> {
        #[serde(borrow)]
        pub server_keys: BTreeMap<&'a Id<ServerName>, BTreeMap<&'a Id<Key>, TimeStamp>>,
    }

    #[derive(Serialize)]
    pub(crate) struct Response<'a> {
        #[serde(skip_serializing_if = "Vec::is_empty")]
        pub server_keys: Vec<&'a RenderedJson<'a, crate::server_keys::ServerKeys>>,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct ServerKeys<'a> {
        #[serde(borrow, skip_serializing_if = "Option::is_none")]
        pub old_verify_keys: Option<BTreeMap<&'a Id<Key>, OldVerifyKey<'a>>>,
        pub server_name: &'a str,
        #[serde(skip_serializing_if = "Option::is_none")]
        pub signatures: Option<Signatures<'a>>,
        pub valid_until_ts: Option<TimeStamp>,
        #[serde(borrow, skip_serializing_if = "BTreeMap::is_empty")]
        pub verify_keys: BTreeMap<&'a Id<Key>, VerifyKey<'a>>,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct OldVerifyKey<'a> {
        pub expired_ts: TimeStamp,
        pub key: &'a str,
    }

    #[derive(Serialize, Deserialize)]
    pub(crate) struct VerifyKey<'a> {
        pub key: &'a str,
    }

    #[derive(Serialize, Deserialize)]
    #[serde(transparent)]
    pub(crate) struct Signatures<'a> {
        #[serde(borrow)]
        pub signatures: BTreeMap<&'a Id<ServerName>, BTreeMap<&'a Id<Key>, String>>,
    }
}

/*
pub(crate) mod get_federation_v1_state {
    use super::*;

    pub(crate) struct RequestPath<'a> {
        room_id: &'a Id<Room>,
    }

    pub(crate) struct RequestQueryString<'a> {
        event_id: &'a Id<Event>,
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
*/
