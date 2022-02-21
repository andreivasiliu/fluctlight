use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    matrix_types::{Id, Key, ServerName},
    rendered_json::RenderedJson,
    request::{EmptyQS, GenericRequest, MatrixRequest, RequestData},
    state::TimeStamp,
};

type Request<'a> = GenericRequest<RequestPath<'a>, EmptyQS, RequestBody<'a>>;

impl<'a> MatrixRequest for Request<'a> {
    type Response = Response<'a>;

    const PATH_SPEC: &'static str = "/_matrix/key/:version/query";
}

#[derive(Serialize, Deserialize)]
pub(super) struct RequestPath<'a> {
    version: &'a str,
}

#[derive(Serialize, Deserialize)]
pub(super) struct RequestBody<'a> {
    #[serde(borrow)]
    server_keys: BTreeMap<&'a Id<ServerName>, BTreeMap<&'a Id<Key>, TimeStamp>>,
}

#[derive(Serialize)]
pub(super) struct Response<'a> {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    server_keys: Vec<&'a RenderedJson<'a, crate::server_keys::ServerKeys>>,
}

#[derive(Serialize, Deserialize)]
struct ServerKeys<'a> {
    #[serde(borrow, skip_serializing_if = "Option::is_none")]
    old_verify_keys: Option<BTreeMap<&'a Id<Key>, OldVerifyKey<'a>>>,
    server_name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    signatures: Option<Signatures<'a>>,
    valid_until_ts: Option<TimeStamp>,
    #[serde(borrow, skip_serializing_if = "BTreeMap::is_empty")]
    verify_keys: BTreeMap<&'a Id<Key>, VerifyKey<'a>>,
}

#[derive(Serialize, Deserialize)]
struct OldVerifyKey<'a> {
    expired_ts: TimeStamp,
    key: &'a str,
}

#[derive(Serialize, Deserialize)]
struct VerifyKey<'a> {
    key: &'a str,
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
struct Signatures<'a> {
    #[serde(borrow)]
    signatures: BTreeMap<&'a Id<ServerName>, BTreeMap<&'a Id<Key>, String>>,
}

pub(super) fn post_key_v2_query<'r>(
    request_data: &RequestData<'r>,
    request: Request<'r>,
) -> Response<'r> {
    let mut server_keys = Vec::new();

    for (server_name, key_query) in request.body.server_keys {
        // Ignore the query. The spec says: "The notary server may return
        // multiple keys egardless of the Key IDs given."
        let _ = key_query;

        if let Some(foreign_server_keys_json) =
            request_data.state.foreign_server_keys_json.get(server_name)
        {
            server_keys.push(foreign_server_keys_json);
        }
    }

    Response { server_keys }
}
