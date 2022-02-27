use std::collections::BTreeMap;

/// GET /_matrix/federation/v1/version
use serde::{Deserialize, Serialize};

use crate::{
    matrix_types::{Device, Id, User},
    request::{EmptyBody, EmptyQS, GenericRequest, MatrixRequest, RequestData},
    server_keys::Signatures,
};

type Request<'a> = GenericRequest<RequestPath<'a>, EmptyQS, EmptyBody>;

impl<'a> MatrixRequest for Request<'a> {
    type Response = Response<'a>;

    const PATH_SPEC: &'static str = "/_matrix/federation/:version/user/devices/:user_id";
}

#[derive(Serialize, Deserialize)]
pub(super) struct RequestPath<'a> {
    #[serde(borrow)]
    version: &'a str,
    user_id: &'a Id<User>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Response<'a> {
    #[serde(borrow)]
    devices: Vec<UserDevice<'a>>,
    master_key: Option<CrossSigningKey<'a>>,
    self_signing_key: Option<CrossSigningKey<'a>>,
    stream_id: i64,
    user_id: &'a Id<User>,
}

#[derive(Serialize, Deserialize)]
struct UserDevice<'a> {
    device_display_name: &'a str,
    device_id: &'a Id<Device>,
    keys: DeviceKeys<'a>,
}

#[derive(Serialize, Deserialize)]
struct DeviceKeys<'a> {
    #[serde(borrow)]
    algorithms: Vec<&'a str>,
    device_id: &'a Id<Device>,
    keys: BTreeMap<&'a str, &'a str>,
    signatures: Signatures,
    user_id: &'a Id<User>,
}

#[derive(Serialize, Deserialize)]
struct CrossSigningKey<'a> {
    #[serde(borrow)]
    keys: BTreeMap<&'a str, &'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    signatures: Option<Signatures>,
    usage: Vec<&'a str>,
    user_id: &'a Id<User>,
}

pub(super) fn get_federation_v1_user_devices<'r>(
    _request_data: &RequestData<'r>,
    request: Request<'r>,
) -> Response<'r> {
    Response {
        devices: vec![],
        master_key: None,
        self_signing_key: None,
        stream_id: 1,
        user_id: request.path.user_id,
    }
}
