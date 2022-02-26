/// GET /_matrix/federation/v1/state/{roomId}
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::request::{EmptyBody, GenericRequest, MatrixRequest, RequestData};

type Request<'a> = GenericRequest<RequestPath<'a>, RequestQueryString<'a>, EmptyBody>;

impl<'a> MatrixRequest for Request<'a> {
    type Response = Response<'a>;

    const PATH_SPEC: &'static str = "/_matrix/federation/:version/state/:room_id";
}

#[derive(Serialize, Deserialize)]
pub(super) struct RequestPath<'a> {
    #[serde(borrow)]
    version: &'a str,
    room_id: &'a str,
}

#[derive(Serialize, Deserialize)]
pub(super) struct RequestQueryString<'a> {
    #[serde(borrow)]
    event_id: &'a str,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Response<'a> {
    #[serde(borrow)]
    auth_chain: Vec<&'a RawValue>,
    pdus: Vec<&'a RawValue>,
}

pub(super) fn get_federation_v1_state<'r>(
    _request_data: &RequestData<'r>,
    _request: Request<'r>,
) -> Response<'r> {
    Response {
        auth_chain: vec![],
        pdus: vec![],
    }
}
