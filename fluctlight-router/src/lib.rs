use fluctlight_mod_interface::{Request, Response, ResponseResult};
use smallvec::SmallVec;

#[no_mangle]
pub extern "C" fn process_request<'a>(request: Request<'a>) -> ResponseResult {
    eprintln!("Hello: {}", request.uri());

    try_process_request(request).into()
}

fn try_process_request<'a>(request: Request<'a>) -> Result<Response, String> {
    let mut uri_segments: SmallVec<[&str; 8]> = request.uri().split('/').collect();
    uri_segments[0] = request.method();

    let (status, body) = match uri_segments.as_slice() {
        ["GET", "_matrix", "federation", "v1", "version"] => (200, "Version!\n"),
        ["GET", "hello"] => (200, "Hello world\n"),
        _ => (404, "Not found\n"),
    };

    Ok(Response::new(status, body.into()))
}
