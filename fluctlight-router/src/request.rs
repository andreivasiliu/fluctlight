use bumpalo::Bump;
use serde::{Serialize, Deserialize};

use crate::{state::State, rest_api_types::MatrixRequest};

pub(crate) struct RequestData<'a> {
    pub memory_pool: &'a Bump,
    pub state: &'a State,
    pub http_request: http::Request<&'a [u8]>,
}

type BumpString<'a> = bumpalo::collections::String<'a>;

impl<'r> RequestData<'r> {
    pub fn new_str(&self, s: &str) -> &'r str {
        BumpString::from_str_in(s, self.memory_pool).into_bump_str()
    }

    pub fn handle_with<F, R>(&self, handler: F) -> Result<http::Response<Vec<u8>>, String>
    where
        F: Fn(&RequestData<'r>, R) -> R::Response,
        R: MatrixRequest + Deserialize<'r>,
        R::Response: Serialize,
    {
        let body = if self.http_request.method() == "GET" {
            b"{}".as_slice()
        } else {
            self.http_request.body()
        };
        let request = serde_json::from_slice(body)
        .map_err(|err| format!("Could not deserialize request: {}", err))?;
        let response = handler(self, request);
        let mut response_bytes = serde_json::to_vec(&response)
            .map_err(|err| format!("Could not serialize response: {}", err))?;
        response_bytes.push(b'\n');
        let http_response = http::Response::builder()
            .body(response_bytes)
            .expect("Response should always be valid");
        Ok(http_response)
    }
}
