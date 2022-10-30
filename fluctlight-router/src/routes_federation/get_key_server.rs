/// GET /_matrix/key/v1/server/{keyId}
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::{
    matrix_types::{Id, Key},
    request::{EmptyBody, EmptyQS, GenericRequest, MatrixRequest, RequestData},
};

type Request<'a> = GenericRequest<RequestPath<'a>, EmptyQS, EmptyBody>;

impl<'a> MatrixRequest for Request<'a> {
    type Response = Response;

    const PATH_SPEC: &'static str = "/_matrix/key/:version/server/?key_id";
}

#[derive(Serialize, Deserialize)]
pub(super) struct RequestPath<'a> {
    version: &'a str,
    #[serde(borrow)]
    key_id: Option<&'a Id<Key>>,
}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub(super) struct Response {
    prerendered_response: Box<RawValue>,
}

pub(super) fn get_key_v2_server<'r>(
    request_data: &RequestData<'r>,
    _request: Request<'r>,
) -> Response {
    // Note: According to the spec, filtering by the key_id query parameter is
    // deprecated, and servers should always return all keys.
    let prerendered_response = request_data.state.render_own_server_keys();

    Response { prerendered_response }
}
