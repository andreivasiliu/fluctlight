// use ruma_federation_api::discovery::{
//     get_server_keys,
//     get_server_version::{self, v1::Server},
// };

use std::collections::BTreeMap;

use crate::{
    request::RequestData,
    rest_api_types::{get_federation_v1_version, get_key_v2_server, post_key_v2_query},
};

pub(super) fn federation_api_handler<'r, 'h>(
    uri_segments: &[&str],
    request_data: &RequestData<'r>,
) -> Option<Result<http::Response<Vec<u8>>, String>> {
    let req = request_data;

    let response_body = match uri_segments {
        ["GET", "_matrix", "federation", _v1, "version"] => {
            req.handle_with(get_federation_v1_version)
        }
        ["GET", "_matrix", "key", "v2", "server", ..] => req.handle_with(get_key_v2_server),
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
            // req.handle_with(post_federation_v2_user_keys_query)
            todo!()
        }

        _ => return None,
    };

    Some(response_body)
}

// GET /_matrix/federation/v1/version
pub(super) fn get_federation_v1_version<'r>(
    request_data: &RequestData<'r>,
    request: get_federation_v1_version::Request<'r>,
) -> get_federation_v1_version::Response<'r> {
    if request.path.version != "v1" {
        eprintln!(
            "Unrecognized API path version: /federation/{}/version",
            request.path.version
        );
    }

    get_federation_v1_version::Response {
        server: get_federation_v1_version::Server {
            name: request_data.new_str("fluctlight"),
            version: request_data.new_str(env!("CARGO_PKG_VERSION")),
        },
    }
}

pub(super) fn get_key_v2_server<'r>(
    request_data: &RequestData<'r>,
    request: get_key_v2_server::Request<'r>,
) -> get_key_v2_server::Response<'r> {
    let mut verify_keys = BTreeMap::new();

    for (key_name, server_key) in &request_data.state.server_keys {
        if request.path.key_id.is_none() || Some(&**key_name) == request.path.key_id {
            verify_keys.insert(
                &**key_name,
                get_key_v2_server::VerifyKey {
                    key: &server_key.public_key_base64,
                },
            );
        }
    }

    let valid_until_ts = request_data
        .state
        .server_keys
        .values()
        .map(|server_key| server_key.valid_until)
        .min_by_key(|timestamp| timestamp.as_secs());

    let mut response = get_key_v2_server::Response {
        old_verify_keys: BTreeMap::new(),
        server_name: &*request_data.state.server_name,
        signatures: None,
        valid_until_ts,
        verify_keys,
    };

    let response_bytes =
        serde_json::to_vec(&response).expect("Serialization should always succeed");
    let mut server_signatures = BTreeMap::new();

    for (key_name, server_key) in &request_data.state.server_keys {
        let noise = None;
        let signature = server_key.key_pair.sk.sign(&response_bytes, noise);
        let sig_b64 = base64::encode_config(&*signature, base64::STANDARD_NO_PAD);

        server_signatures.insert(&**key_name, sig_b64);
    }

    let mut signatures = BTreeMap::new();
    signatures.insert(&*request_data.state.server_name, server_signatures);

    response.signatures = Some(get_key_v2_server::Signatures { signatures });

    response
}

pub(super) fn post_key_v2_query<'r>(
    request_data: &RequestData<'r>,
    request: post_key_v2_query::Request<'r>,
) -> post_key_v2_query::Response<'r> {
    let mut verify_keys = BTreeMap::new();

    for (server_name, key_query) in request.body.server_keys {
        if server_name == &*request_data.state.server_name {
        } else {
        }
    }

    let valid_until_ts = request_data
        .state
        .server_keys
        .values()
        .map(|server_key| server_key.valid_until)
        .min_by_key(|timestamp| timestamp.as_secs());

    let mut own_server_keys = post_key_v2_query::ServerKeys {
        old_verify_keys: Some(BTreeMap::new()),
        server_name: "fluctlight-dev.demi.ro",
        signatures: None,
        valid_until_ts,
        verify_keys,
    };

    let response_bytes =
        serde_json::to_vec(&own_server_keys).expect("Serialization should always succeed");
    let mut server_signatures = BTreeMap::new();

    for (key_name, server_key) in &request_data.state.server_keys {
        let noise = None;
        let signature = server_key.key_pair.sk.sign(&response_bytes, noise);
        let sig_b64 = base64::encode_config(&*signature, base64::STANDARD_NO_PAD);

        server_signatures.insert(&**key_name, sig_b64);
    }

    let mut signatures = BTreeMap::new();
    signatures.insert(&*request_data.state.server_name, server_signatures);

    own_server_keys.signatures = Some(post_key_v2_query::Signatures { signatures });

    let mut server_keys = Vec::new();
    server_keys.push(own_server_keys);

    post_key_v2_query::Response { server_keys }
}

/*
// POST /_matrix/federation/v2/user/keys/query
pub(super) fn post_federation_v2_user_keys_query<'r>(
    request_data: &RequestData<'r>,
    _request: rest_api_types::post_federation_v1_user_keys_query::Request<'r>,
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
*/
