use crate::request::RequestData;

use self::get_send::get_admin_send;

mod get_send;

pub(super) fn admin_api_handler<'r, 'h>(
    uri_segments: &[&str],
    request_data: &RequestData<'r>,
) -> Option<Result<http::Response<Vec<u8>>, String>> {
    let req = request_data;

    let response_body = match uri_segments {
        ["GET", "admin", "send"] => req.handle_with(get_admin_send),

        _ => return None,
    };

    Some(response_body)
}
