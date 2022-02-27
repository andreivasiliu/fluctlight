use askama::Template;
/// GET /_matrix/federation/v1/version
use serde::{Deserialize, Serialize};

use crate::{
    request::{EmptyBody, EmptyQS, GenericRequest, MatrixRequest, RequestData},
    state::State,
};

type Request<'a> = GenericRequest<RequestPath<'a>, EmptyQS, EmptyBody>;

impl<'a> MatrixRequest for Request<'a> {
    type Response = Response<'a>;

    const PATH_SPEC: &'static str = "/admin/view";
}

#[derive(Serialize, Deserialize)]
pub(super) struct RequestPath<'a> {
    #[serde(skip)]
    phantom: std::marker::PhantomData<&'a ()>,
}

#[derive(Template)]
#[template(path = "view.html")]
pub(super) struct Response<'a> {
    state: &'a State,
}

pub(super) fn get_admin_view<'r>(
    request_data: &RequestData<'r>,
    _request: Request<'r>,
) -> Response<'r> {
    Response {
        state: request_data.state,
    }
}
