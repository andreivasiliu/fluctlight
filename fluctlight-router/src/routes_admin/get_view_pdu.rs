use askama::Template;
/// GET /_matrix/federation/v1/version
use serde::{Deserialize, Serialize};

use crate::{
    matrix_types::{Event, Id, Room},
    request::{EmptyBody, EmptyQS, GenericRequest, MatrixRequest, RequestData},
    state::State,
};

type Request<'a> = GenericRequest<RequestPath<'a>, EmptyQS, EmptyBody>;

impl<'a> MatrixRequest for Request<'a> {
    type Response = Response<'a>;

    const PATH_SPEC: &'static str = "/admin/view/pdu/:room_id/:event_id";
}

#[derive(Serialize, Deserialize)]
pub(super) struct RequestPath<'a> {
    #[serde(borrow)]
    room_id: &'a Id<Room>,
    event_id: &'a Id<Event>,
}

#[derive(Template)]
#[template(path = "view_pdu.html")]
pub(super) struct Response<'a> {
    room_id: &'a Id<Room>,
    event_id: &'a Id<Event>,
    state: &'a State,
}

pub(super) fn get_admin_view_pdu<'r>(
    request_data: &RequestData<'r>,
    request: Request<'r>,
) -> Response<'r> {
    Response {
        room_id: request.path.room_id,
        event_id: request.path.event_id,
        state: request_data.state,
    }
}
