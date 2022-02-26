/// GET /_matrix/federation/v1/version
use serde::{Deserialize, Serialize};

use crate::{
    matrix_types::{Event, Id, Room, ServerName, User},
    request::{EmptyBody, EmptyQS, GenericRequest, MatrixRequest, RequestData},
    send::send_request,
    state::TimeStamp,
};

type Request<'a> = GenericRequest<RequestPath<'a>, EmptyQS, EmptyBody>;

impl<'a> MatrixRequest for Request<'a> {
    type Response = Response<'a>;

    const PATH_SPEC: &'static str = "/admin/send";
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

#[derive(Serialize, Deserialize)]
struct Server<'a> {
    name: &'a str,
    version: &'a str,
}

pub(super) fn get_admin_send<'r>(
    request_data: &RequestData<'r>,
    _request: Request<'r>,
) -> Response<'r> {
    let text = request_data.new_str("Hello");

    match send_request(&request_data.state) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("Error: {}", err);
        }
    };

    Response { text }
}
