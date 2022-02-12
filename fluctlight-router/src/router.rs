use std::borrow::Cow;

use fluctlight_mod_interface::{Request, Response};
use ruma_api::{IncomingNonAuthRequest, IncomingRequest, OutgoingResponse};
use smallvec::SmallVec;

use crate::{routes_federation::federation_uri_handler, State};

pub(super) fn try_process_request<'a>(
    state: &State,
    request: Request<'a>,
) -> Result<Response, String> {
    let mut uri_segments: SmallVec<[&str; 8]> = request.uri().split('/').collect();
    uri_segments[0] = request.method();

    let function = if let Some(function) = federation_uri_handler(uri_segments.as_slice()) {
        function
    } else {
        return Ok(Response::new(404, b"Not found\n".as_slice().into()));
    };

    let request = http::Request::builder()
        .method(request.method())
        .uri(request.uri())
        .body(Cow::Borrowed(b"".as_slice()))
        .expect("Request should always be valid");

    let response = function.handle_route(state, request)?;

    return Ok(Response::new(200, response.into()));
}

pub(crate) struct Auth;

pub(crate) trait MatrixRoute {
    fn handle_route(
        &self,
        state: &State,
        request: http::Request<Cow<'_, [u8]>>,
    ) -> Result<Vec<u8>, String>;
}

impl<Req: IncomingNonAuthRequest> MatrixRoute for fn(&State, Req) -> Req::OutgoingResponse {
    fn handle_route(
        &self,
        state: &State,
        request: http::Request<Cow<'_, [u8]>>,
    ) -> Result<Vec<u8>, String> {
        let request = Req::try_from_http_request(request)
            .map_err(|err| format!("Could not deserialize request: {}", err))?;
        let response = self(state, request);
        let response = response
            .try_into_http_response()
            .map_err(|err| format!("Could not serialize response: {}", err))?;
        let mut body: Vec<u8> = response.into_body();
        body.push(b'\n');
        Ok(body)
    }
}

impl<Req: IncomingRequest> MatrixRoute for fn(&State, Auth, Req) -> Req::OutgoingResponse {
    fn handle_route(
        &self,
        state: &State,
        request: http::Request<Cow<'_, [u8]>>,
    ) -> Result<Vec<u8>, String> {
        let request = Req::try_from_http_request(request)
            .map_err(|err| format!("Could not deserialize request: {}", err))?;
        let response = self(state, Auth, request);
        let response = response
            .try_into_http_response()
            .map_err(|err| format!("Could not serialize response: {}", err))?;
        let mut body: Vec<u8> = response.into_body();
        body.push(b'\n');
        Ok(body)
    }
}
