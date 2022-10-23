/// GET /_matrix/federation/v1/version
use serde::{Deserialize, Serialize};

use crate::{
    playground::send_backfill_request,
    request::{EmptyBody, EmptyQS, GenericRequest, MatrixRequest, RequestData},
};

type Request<'a> = GenericRequest<RequestPath<'a>, EmptyQS, EmptyBody>;

impl<'a> MatrixRequest for Request<'a> {
    type Response = Response<'a>;

    const PATH_SPEC: &'static str = "/admin/backfill";
}

#[derive(Serialize, Deserialize)]
pub(super) struct RequestPath<'a> {
    #[serde(skip)]
    phantom: std::marker::PhantomData<&'a ()>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Response<'a> {
    text: &'a str,
}

pub(super) fn get_admin_backfill<'r>(
    request_data: &RequestData<'r>,
    _request: Request<'r>,
) -> Response<'r> {
    let text = if let Err(err) = send_backfill_request(request_data.state) {
        bumpalo::format!(in request_data.memory_pool, "Error: {}", err).into_bump_str()
    } else {
        request_data.new_str("Backfill successful.")
    };

    Response { text }
}
