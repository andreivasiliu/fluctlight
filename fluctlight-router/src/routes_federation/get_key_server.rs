/// GET /_matrix/key/v1/server/{keyId}
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    matrix_types::{Id, Key, ServerName},
    request::{EmptyBody, EmptyQS, GenericRequest, MatrixRequest, RequestData},
    state::TimeStamp,
};

type Request<'a> = GenericRequest<RequestPath<'a>, EmptyQS, EmptyBody>;

impl<'a> MatrixRequest for Request<'a> {
    type Response = Response<'a>;

    const PATH_SPEC: &'static str = "/_matrix/key/:version/server/?key_id";
}

#[derive(Serialize, Deserialize)]
pub(super) struct RequestPath<'a> {
    version: &'a str,
    #[serde(borrow)]
    key_id: Option<&'a Id<Key>>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Response<'a> {
    #[serde(borrow)]
    old_verify_keys: Option<BTreeMap<&'a Id<Key>, OldVerifyKey<'a>>>,
    server_name: &'a Id<ServerName>,
    #[serde(skip_serializing_if = "Option::is_none")]
    signatures: Option<Signatures<'a>>,
    valid_until_ts: Option<TimeStamp>,
    verify_keys: Option<BTreeMap<&'a Id<Key>, VerifyKey<'a>>>,
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

pub(super) fn get_key_v2_server<'r>(
    request_data: &RequestData<'r>,
    request: Request<'r>,
) -> Response<'r> {
    let mut verify_keys = BTreeMap::new();

    // TODO: Change to prerendered JSON
    for (key_name, server_key) in &request_data.state.server_key_pairs {
        if request.path.key_id.is_none() || Some(&**key_name) == request.path.key_id {
            verify_keys.insert(
                &**key_name,
                VerifyKey {
                    key: &server_key.public_key_base64,
                },
            );
        }
    }

    let valid_until_ts = request_data
        .state
        .server_key_pairs
        .values()
        .map(|server_key| server_key.valid_until)
        .min_by_key(|timestamp| timestamp.as_millis())
        .expect("Server should always have at least one key");

    let mut response = Response {
        old_verify_keys: Some(BTreeMap::new()),
        server_name: &*request_data.state.server_name,
        signatures: None,
        valid_until_ts: Some(valid_until_ts),
        verify_keys: Some(verify_keys),
    };

    let response_bytes =
        serde_json::to_vec(&response).expect("Serialization should always succeed");
    let mut server_signatures = BTreeMap::new();

    eprintln!(
        "Signing key response bytes:\n---\n{}\n---\n",
        String::from_utf8_lossy(&response_bytes)
    );

    for (key_name, server_key) in &request_data.state.server_key_pairs {
        let noise = None;
        let signature = server_key.key_pair.sk.sign(&response_bytes, noise);
        let sig_b64 = base64::encode_config(&*signature, base64::STANDARD_NO_PAD);

        server_signatures.insert(&**key_name, sig_b64);
    }

    let mut signatures = BTreeMap::new();
    signatures.insert(&*request_data.state.server_name, server_signatures);

    response.signatures = Some(Signatures { signatures });

    response
}
