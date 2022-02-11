use fluctlight_mod_interface::{Request, Response};

#[no_mangle]
pub extern "C" fn process_request<'a>(request: Request<'a>) -> Response {
    eprintln!("Hello: {}", request.uri());

    Response::new(200, "Ok\n".into())
}
