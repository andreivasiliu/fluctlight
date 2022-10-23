use crate::request::RequestData;

use self::{
    get_key_server::get_key_v2_server, get_state::get_federation_v1_state,
    get_user_devices::get_federation_v1_user_devices, get_version::get_federation_v1_version,
    post_key_query::post_key_v2_query, put_send::put_federation_v1_send,
};

mod get_key_server;
mod get_state;
mod get_user_devices;
mod get_version;
mod post_key_query;
mod put_send;

pub(super) fn federation_api_handler<'r, 'h>(
    uri_segments: &[&str],
    request_data: &RequestData<'r>,
) -> Option<Result<http::Response<Vec<u8>>, String>> {
    let req = request_data;

    let response_body = match uri_segments {
        ["GET", ".well-known", "matrix", "server"] => not_implemented(), // important
        ["GET", "_matrix", "federation", _v1, "version"] => {
            req.handle_with(get_federation_v1_version)
        }
        ["GET", "_matrix", "key", "v2", "server", ..] => req.handle_with(get_key_v2_server),
        ["POST", "_matrix", "key", "v2", "query"] => req.handle_with(post_key_v2_query),
        ["GET", "_matrix", "key", "v2", "query", _, _] => not_implemented(),
        ["PUT", "_matrix", "federation", "v1", "send", _] => {
            req.handle_with(put_federation_v1_send)
        }
        ["GET", "_matrix", "federation", "v1", "event_auth", _, _] => not_implemented(),
        ["GET", "_matrix", "federation", "v1", "backfill", _] => not_implemented(),
        ["POST", "_matrix", "federation", "v1", "get_missing_events", _] => not_implemented(),
        ["GET", "_matrix", "federation", "v1", "event", _] => not_implemented(),
        ["GET", "_matrix", "federation", "v1", "state", _] => {
            req.handle_with(get_federation_v1_state)
        }
        ["GET", "_matrix", "federation", "v1", "state_ids", _] => not_implemented(),
        ["GET", "_matrix", "federation", "v1", "make_join", _, _] => not_implemented(),
        ["PUT", "_matrix", "federation", "v1", "send_join", _, _] => not_implemented(),
        ["PUT", "_matrix", "federation", "v2", "send_join", _, _] => not_implemented(),
        ["GET", "_matrix", "federation", "v1", "make_knock", _, _] => not_implemented(),
        ["PUT", "_matrix", "federation", "v1", "send_knock", _, _] => not_implemented(),
        ["PUT", "_matrix", "federation", "v1", "invite", _, _] => return None,
        ["PUT", "_matrix", "federation", "v2", "invite", _, _] => not_implemented(),
        ["GET", "_matrix", "federation", "v1", "make_leave", _, _] => not_implemented(),
        ["PUT", "_matrix", "federation", "v1", "send_leave", _, _] => not_implemented(),
        ["PUT", "_matrix", "federation", "v2", "send_leave", _, _] => not_implemented(),
        ["PUT", "_matrix", "federation", "v1", "3pid", "onbind"] => not_implemented(),
        ["PUT", "_matrix", "federation", "v1", "exchange_third_party_invite", _] => {
            not_implemented()
        }
        ["GET", "_matrix", "federation", "v1", "publicRooms"] => not_implemented(),
        ["POST", "_matrix", "federation", "v1", "publicRooms"] => not_implemented(),
        ["GET", "_matrix", "federation", "v1", "hierarchy", _] => not_implemented(),
        ["GET", "_matrix", "federation", "v1", "query", "directory"] => not_implemented(),
        ["GET", "_matrix", "federation", "v1", "query", "profile"] => not_implemented(), // important
        ["GET", "_matrix", "federation", "v1", "query", _] => not_implemented(),
        ["GET", "_matrix", "federation", "v1", "user", "devices", _] => {
            req.handle_with(get_federation_v1_user_devices)
        }
        ["POST", "_matrix", "federation", "v1", "user", "keys", "claim"] => not_implemented(),
        ["POST", "_matrix", "federation", "v1", "user", "keys", "query"] => {
            // req.handle_with(post_federation_v2_user_keys_query)
            not_implemented()
        }

        _ => return None,
    };

    Some(response_body)
}

fn not_implemented() -> Result<http::Response<Vec<u8>>, String> {
    let response = http::Response::builder()
        .status(501)
        .header("Content-Type", "text/plain")
        .body(b"Not yet implemented\n".to_vec())
        .expect("Valid constant response");

    Ok(response)
}
