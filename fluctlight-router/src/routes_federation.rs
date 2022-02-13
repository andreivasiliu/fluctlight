// use ruma_federation_api::discovery::{
//     get_server_keys,
//     get_server_version::{self, v1::Server},
// };

use std::collections::BTreeMap;

use crate::{request::RequestData, rest_api_types};

pub(super) fn federation_api_handler<'r, 'h>(uri_segments: &[&str], request_data: &RequestData<'r>) -> Option<Result<http::Response<Vec<u8>>, String>> {
    let req = request_data;

    let response_body = match uri_segments {
        ["GET", "_matrix", "federation", "v1", "version"] => {
            req.handle_with(get_federation_v1_version)
        }
        ["GET", "_matrix", "key", "v2", "server", ..] => {
            req.handle_with(post_federation_v2_user_keys_query)
        }
        ["POST", "_matrix", "key", "v2", "query"] => todo!(),
        ["GET", "_matrix", "key", "v2", "query", _, _] => todo!(),
        ["PUT", "_matrix", "federation", "v1", "send", _] => todo!(),
        ["GET", "_matrix", "federation", "v1", "event_auth", _, _] => todo!(),
        ["GET", "_matrix", "federation", "v1", "backfill", _] => todo!(),
        ["POST", "_matrix", "federation", "v1", "get_missing_events", _] => todo!(),
        ["GET", "_matrix", "federation", "v1", "event", _] => todo!(),
        ["GET", "_matrix", "federation", "v1", "state", _] => todo!(),
        ["GET", "_matrix", "federation", "v1", "state_ids", _] => todo!(),
        ["GET", "_matrix", "federation", "v1", "make_join", _, _] => todo!(),
        ["PUT", "_matrix", "federation", "v1", "send_join", _, _] => todo!(),
        ["PUT", "_matrix", "federation", "v2", "send_join", _, _] => todo!(),
        ["GET", "_matrix", "federation", "v1", "make_knock", _, _] => todo!(),
        ["PUT", "_matrix", "federation", "v1", "send_knock", _, _] => todo!(),
        ["PUT", "_matrix", "federation", "v1", "invite", _, _] => return None,
        ["PUT", "_matrix", "federation", "v2", "invite", _, _] => todo!(),
        ["GET", "_matrix", "federation", "v1", "make_leave", _, _] => todo!(),
        ["PUT", "_matrix", "federation", "v1", "send_leave", _, _] => todo!(),
        ["PUT", "_matrix", "federation", "v2", "send_leave", _, _] => todo!(),
        ["PUT", "_matrix", "federation", "v1", "3pid", "onbind"] => todo!(),
        ["PUT", "_matrix", "federation", "v1", "exchange_third_party_invite", _] => todo!(),
        ["GET", "_matrix", "federation", "v1", "publicRooms"] => todo!(),
        ["POST", "_matrix", "federation", "v1", "publicRooms"] => todo!(),
        ["GET", "_matrix", "federation", "v1", "hierarchy", _] => todo!(),
        ["GET", "_matrix", "federation", "v1", "query", "directory"] => todo!(),
        ["GET", "_matrix", "federation", "v1", "query", "profile"] => todo!(),
        ["GET", "_matrix", "federation", "v1", "query", _] => todo!(),
        ["GET", "_matrix", "federation", "v1", "user", "devices", _] => todo!(),
        ["POST", "_matrix", "federation", "v1", "user", "keys", "claim"] => todo!(),
        ["POST", "_matrix", "federation", "v1", "user", "keys", "query"] => {
            req.handle_with(post_federation_v2_user_keys_query)
        },

        _ => return None,
    };

    Some(response_body)
}

// GET /_matrix/federation/v1/version
pub(super) fn get_federation_v1_version<'r>(
    request_data: &RequestData<'r>,
    _request: rest_api_types::get_federation_v1_version::RequestBody<'r>,
) -> rest_api_types::get_federation_v1_version::Response<'r> {
    rest_api_types::get_federation_v1_version::Response {
        server: rest_api_types::get_federation_v1_version::Server {
            name: request_data.new_str("fluctlight"),
            version: request_data.new_str("0.0.1-alpha.5"),
        },
    }
}

// POST /_matrix/federation/v2/user/keys/query
pub(super) fn post_federation_v2_user_keys_query<'r>(
    request_data: &RequestData<'r>,
    _request: rest_api_types::post_federation_v1_user_keys_query::RequestBody<'r>,
) -> rest_api_types::post_federation_v1_user_keys_query::Response<'r> {
    let mut device_keys = BTreeMap::new();

    for (user, user_state) in &request_data.state.users {
        let mut user_device_keys = BTreeMap::new();

        for (device, _key) in &user_state.keys {
            user_device_keys.insert(&**device, rest_api_types::post_federation_v1_user_keys_query::DeviceKeys {
                user_id: &**user, device_id: &**device, algorithms: vec![],
            });

        }
        device_keys.insert(&**user, user_device_keys);
    }

    rest_api_types::post_federation_v1_user_keys_query::Response {
        device_keys,
    }
}
