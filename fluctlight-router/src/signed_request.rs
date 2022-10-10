use std::error::Error;

use serde::{de::DeserializeOwned, Serialize};
use serde_json::value::{to_raw_value, RawValue};

use crate::state::State;

#[derive(Serialize)]
struct SignedJson<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<&'a RawValue>,
    destination: &'a str,
    method: &'a str,
    origin: &'a str,
    uri: &'a str,
}

pub(crate) struct SignedRequestBuilder<'a> {
    state: &'a State,
    origin: &'a str,
    uri: &'a str,
    method: &'static str,
    destination: Option<&'a str>,
}

// This is awful.
// Maybe have signed_get and signed_post functions?
// signed_post(state, "/abc", destination, value)
// That's also awful. Need a better idea here.
impl<'a> SignedRequestBuilder<'a> {
    pub(crate) fn get(state: &'a State, uri: &'a str) -> Self {
        let origin = state.server_name.as_str();

        SignedRequestBuilder {
            method: "GET",
            state,
            origin,
            uri,
            destination: None,
        }
    }

    pub(crate) fn destination(self, destination: &'a str) -> Self {
        SignedRequestBuilder {
            destination: Some(destination),
            ..self
        }
    }

    pub(crate) fn send<R: DeserializeOwned>(self) -> Result<R, Box<dyn Error>> {
        let signed_json = SignedJson {
            content: None,
            destination: self.destination.expect("Destination must be set"),
            method: self.method,
            origin: self.origin,
            uri: self.uri,
        };

        let url = format!(
            "http://{}:8008{}", signed_json.destination, self.uri,
        );

        let req = match self.method {
            "GET" => ureq::get(&url),
            "POST" => ureq::post(&url),
            method => panic!("Unknown method {}", method),
        };

        let signature = signature(self.state, &signed_json);
        let req = req.set("Authorization", &signature);

        let response = req.call()?;

        Ok(serde_json::from_reader(response.into_reader())?)
    }

    pub(crate) fn send_body<T: Serialize, R: DeserializeOwned>(
        self,
        body: &T,
    ) -> Result<R, Box<dyn Error>> {
        let req = match self.method {
            "GET" => ureq::get(self.uri),
            "POST" => ureq::post(self.uri),
            method => panic!("Unknown method {}", method),
        };

        let content = to_raw_value(body)?;

        let signed_json = SignedJson {
            content: Some(&content),
            destination: self.destination.expect("Destination must be set"),
            method: self.method,
            origin: self.origin,
            uri: self.uri,
        };

        let signature = signature(self.state, &signed_json);
        let req = req.set("Authorization", &signature);

        let response = req.send_bytes(content.get().as_bytes())?;

        Ok(serde_json::from_reader(response.into_reader())?)
    }
}

fn signature(state: &State, signed_json: &SignedJson) -> String {
    // TODO: Check if sending directly to a hasher helps
    let signable_bytes = serde_json::to_vec(&signed_json).unwrap();

    eprintln!("Signing signable:\n---\n{}\n---\n", String::from_utf8_lossy(&signable_bytes));

    let server_key = state.server_key_pairs.iter().next().unwrap();
    let (key_name, server_key_pair) = server_key;

    let noise = None;
    let signature = server_key_pair.key_pair.sk.sign(&signable_bytes, noise);
    let sig_b64 = base64::encode_config(&*signature, base64::STANDARD_NO_PAD);

    let sig = format!(
        "X-Matrix origin={},key=\"{}\",sig=\"{}\"",
        state.server_name, key_name, sig_b64
    );

    eprintln!("Signature: {}", sig);
    sig
}
