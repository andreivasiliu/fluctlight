use std::error::Error;

use serde::Serialize;
use serde_json::value::RawValue;

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
// Either way this module doesn't belong in this library.
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

    pub(crate) fn put(state: &'a State, uri: &'a str) -> Self {
        let origin = state.server_name.as_str();

        SignedRequestBuilder {
            method: "PUT",
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

    pub(crate) fn send(self) -> Result<Vec<u8>, Box<dyn Error>> {
        let signed_json = SignedJson {
            content: None,
            destination: self.destination.expect("Destination must be set"),
            method: self.method,
            origin: self.origin,
            uri: self.uri,
        };

        let url = format!("http://{}:8008{}", signed_json.destination, self.uri);

        let mut req = match self.method {
            "GET" => ureq::get(&url),
            "POST" => ureq::post(&url),
            method => panic!("Unknown method {}", method),
        };

        req = sign(req, self.state, &signed_json);

        let response = req.call()?;
        let mut bytes = Vec::new();
        response.into_reader().read_to_end(&mut bytes)?;

        Ok(bytes)
    }

    pub(crate) fn send_body(self, content: Box<RawValue>) -> Result<Vec<u8>, Box<dyn Error>> {
        let signed_json = SignedJson {
            content: Some(&content),
            destination: self.destination.expect("Destination must be set"),
            method: self.method,
            origin: self.origin,
            uri: self.uri,
        };

        let url = format!("http://{}:8008{}", signed_json.destination, self.uri);

        let mut req = match self.method {
            "GET" => ureq::get(&url),
            "POST" => ureq::post(&url),
            "PUT" => ureq::put(&url),
            method => panic!("Unknown method {}", method),
        };

        req = sign(req, self.state, &signed_json);

        let response = req.send_bytes(content.get().as_bytes())?;
        let mut bytes = Vec::new();
        response.into_reader().read_to_end(&mut bytes)?;

        Ok(bytes)
    }
}

fn sign(mut req: ureq::Request, state: &State, signed_json: &SignedJson) -> ureq::Request {
    // TODO: Check if sending directly to a hasher helps
    let signable_bytes = serde_json::to_vec(&signed_json).unwrap();

    eprintln!(
        "Signing signable:\n---\n{}\n---\n",
        String::from_utf8_lossy(&signable_bytes)
    );

    for (key_name, server_key) in &state.server_key_pairs {
        let noise = None;
        let signature = server_key.key_pair.sk.sign(&signable_bytes, noise);
        let sig_b64 = base64::encode_config(&*signature, base64::STANDARD_NO_PAD);

        let header = format!(
            "X-Matrix origin={},key=\"{}\",sig=\"{}\"",
            state.server_name, key_name, sig_b64
        );

        req = req.set("Authorization", &header);
    }

    req
}
