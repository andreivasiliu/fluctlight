use crate::request::RequestData;

use self::{
    get_load::get_admin_load, get_send::get_admin_send, get_view::get_admin_view,
    get_view_pdu::get_admin_view_pdu,
};

mod get_load;
mod get_send;
mod get_view;
mod get_view_pdu;

pub(super) fn admin_api_handler<'r, 'h>(
    uri_segments: &[&str],
    request_data: &RequestData<'r>,
) -> Option<Result<http::Response<Vec<u8>>, String>> {
    let req = request_data;

    let response_body = match uri_segments {
        ["GET", "admin", "send"] => req.handle_with(get_admin_send),
        ["GET", "admin", "load"] => req.handle_with(get_admin_load),
        ["GET", "admin", "view"] => req.render_template_with(get_admin_view),
        ["GET", "admin", "view", "pdu", _, _] => req.render_template_with(get_admin_view_pdu),

        _ => return None,
    };

    Some(response_body)
}
