use std::collections::BTreeMap;

/// GET /_matrix/federation/v1/version
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::{
    playground::ingest_transaction,
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
    edus: Option<Vec<&'a RawValue>>,
    origin: &'a str,
    origin_server_ts: TimeStamp,
    pdus: Vec<&'a RawValue>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Response<'a> {
    // FIXME: Should be Id<Event> (once borrowing is fixed)
    #[serde(borrow)]
    pdus: BTreeMap<&'a str, PDUProcessingResult<'a>>,
}

#[derive(Serialize, Deserialize)]
struct PDUProcessingResult<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<&'a str>,
}

pub(super) fn put_federation_v1_send<'r>(
    request_data: &RequestData<'r>,
    request: Request<'r>,
) -> Response<'r> {
    let pdus = ingest_transaction(
        request_data.state,
        request.path.transaction_id,
        request.body.origin,
        request.body.origin_server_ts,
        request.body.pdus,
        request.body.edus.unwrap_or(vec![]),
    );

    let pdus: BTreeMap<_, _> = pdus
        .into_iter()
        .map(|(event_id, value)| {
            let event_id = request_data.new_str(event_id.as_str());
            if let Err(err) = value {
                let err = request_data.new_str(err.as_str());
                (event_id, PDUProcessingResult { error: Some(err) })
            } else {
                (event_id, PDUProcessingResult { error: None })
            }
        })
        .collect();

    Response { pdus }
}
