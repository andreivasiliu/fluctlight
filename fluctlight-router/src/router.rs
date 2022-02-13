use fluctlight_mod_interface::{Request, Response};
use smallvec::SmallVec;

use crate::{
    request::RequestData, routes_federation::federation_api_handler,
    state::State,
};

pub(super) fn try_process_request<'a>(
    state: &State,
    request: Request<'a>,
) -> Result<Response, String> {
    let mut uri_segments: SmallVec<[&str; 8]> = request.uri().split('/').collect();
    uri_segments[0] = request.method();

    // let function = if let Some(function) = federation_uri_handler(uri_segments.as_slice()) {
    //     function
    // } else {
    //     return Ok(Response::new(404, b"Not found\n".as_slice().into()));
    // };

    let http_request = http::Request::builder()
        .method(request.method())
        .uri(request.uri())
        .body(b"".as_slice())
        .expect("Request should always be valid");

    let memory_pool = bumpalo::Bump::with_capacity(256);
    let request_data = RequestData {
        memory_pool: &memory_pool,
        state: &state,
        http_request,
    };

    let http_response = if let Some(http_response) = federation_api_handler(uri_segments.as_slice(), &request_data) {
        http_response
    } else if let Some(http_response) = federation_api_handler(uri_segments.as_slice(), &request_data) {
        http_response
    } else {
        return Ok(Response::new(404, b"Not found\n".as_slice().into()));
    };

    let http_response = http_response
        .map_err(|err| format!("Could not process request: {}", err))?;
    return Ok(Response::new(200, http_response.into_body().into()));
}
