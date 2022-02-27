use std::collections::BTreeMap;

/// GET /_matrix/federation/v1/version
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::{
    matrix_types::{Event, Id},
    request::{EmptyQS, GenericRequest, MatrixRequest, RequestData},
    state::TimeStamp,
};

type Request<'a> = GenericRequest<RequestPath<'a>, EmptyQS, RequestBody<'a>>;

impl<'a> MatrixRequest for Request<'a> {
    type Response = Response<'a>;

    const PATH_SPEC: &'static str = "/_matrix/federation/:version/send/:transaction_id";
}

#[derive(Serialize, Deserialize)]
pub(super) struct RequestPath<'a> {
    #[serde(borrow)]
    version: &'a str,
    transaction_id: &'a str,
}

#[derive(Serialize, Deserialize)]
pub(super) struct RequestBody<'a> {
    #[serde(borrow)]
    edus: Vec<&'a RawValue>,
    origin: &'a str,
    origin_server_ts: TimeStamp,
    pdus: Vec<&'a RawValue>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Response<'a> {
    #[serde(borrow)]
    pdus: BTreeMap<&'a Id<Event>, PDUProcessingResult<'a>>,
}

#[derive(Serialize, Deserialize)]
struct PDUProcessingResult<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'a str>,
}

pub(super) fn put_federation_v1_send<'r>(
    _request_data: &RequestData<'r>,
    request: Request<'r>,
) -> Response<'r> {
    eprintln!("On transaction {}:", request.path.transaction_id);
    for pdu in &request.body.pdus {
        eprintln!("Got PDU: {}", pdu)
    }

    for edu in request.body.edus {
        eprintln!("Got EDU: {}", edu);
    }

    if !request.body.pdus.is_empty() {
        todo!("PDU parsing missing");
    }

    Response {
        pdus: BTreeMap::new(),
    }
}
