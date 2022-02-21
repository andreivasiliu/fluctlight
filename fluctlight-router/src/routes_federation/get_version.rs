/// GET /_matrix/federation/v1/version
use serde::{Deserialize, Serialize};

use crate::request::{EmptyBody, EmptyQS, GenericRequest, MatrixRequest, RequestData};

type Request<'a> = GenericRequest<RequestPath<'a>, EmptyQS, EmptyBody>;

impl<'a> MatrixRequest for Request<'a> {
    type Response = Response<'a>;

    const PATH_SPEC: &'static str = "/_matrix/federation/:version/version";
}

#[derive(Serialize, Deserialize)]
pub(super) struct RequestPath<'a> {
    #[serde(borrow)]
    version: &'a str,
}

#[derive(Serialize, Deserialize)]
pub(super) struct Response<'a> {
    #[serde(borrow)]
    server: Server<'a>,
}

#[derive(Serialize, Deserialize)]
struct Server<'a> {
    name: &'a str,
    version: &'a str,
}

pub(super) fn get_federation_v1_version<'r>(
    request_data: &RequestData<'r>,
    request: Request<'r>,
) -> Response<'r> {
    if request.path.version != "v1" {
        eprintln!(
            "Unrecognized API path version: /federation/{}/version",
            request.path.version
        );
    }

    Response {
        server: Server {
            name: request_data.new_str("fluctlight"),
            version: request_data.new_str(env!("CARGO_PKG_VERSION")),
        },
    }
}
