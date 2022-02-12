use ruma_federation_api::discovery::{
    get_server_keys,
    get_server_version::{self, v1::Server},
};

use crate::{router::MatrixRoute, State};

pub(super) fn federation_uri_handler(uri_segments: &[&str]) -> Option<&'static dyn MatrixRoute> {
    let function: &dyn MatrixRoute = match uri_segments {
        ["GET", "_matrix", "federation", "v1", "version"] => {
            &(get_federation_v1_version as fn(&_, _) -> _)
        }
        ["GET", "_matrix", "key", "v2", "server"] => &(get_key_v2_server as fn(&_, _) -> _),
        ["POST", "_matrix", "key", "v2", "query"] => todo!(),

        _ => return None,
    };

    Some(function)
}

// GET /_matrix/federation/v1/version
pub(super) fn get_federation_v1_version(
    _state: &State,
    _request: get_server_version::v1::Request,
) -> get_server_version::v1::Response {
    let mut server = Server::new();
    server.name = Some("fluctlight".to_string());
    server.version = Some("0.0.1-alpha.4".to_string());

    let mut response = get_server_version::v1::Response::new();
    response.server = Some(server);

    response
}

// GET /_matrix/key/v2/server
pub(super) fn get_key_v2_server(
    _state: &State,
    _request: get_server_keys::v2::Request,
) -> get_server_keys::v2::Response {
    todo!()
}
