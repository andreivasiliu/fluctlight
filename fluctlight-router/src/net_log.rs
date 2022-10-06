use std::{borrow::Cow, path::Path};

use http::{Request, Response};
use serde::Serialize;
use serde_json::Value;
use smallvec::SmallVec;

#[derive(Serialize)]
struct RequestEntry<'a> {
    path: &'a str,
    body: Cow<'a, str>,
    json: Option<Value>,
    headers: &'a [(&'a str, Cow<'a, str>)],
}

#[derive(Serialize)]
struct ResponseEntry<'a> {
    body: Cow<'a, str>,
    json: Option<Value>,
    headers: &'a [(&'a str, Cow<'a, str>)],
    status_code: u16,
}

pub(crate) fn log_network_request(log_index: usize, request: &Request<&[u8]>, direction: &str) {
    if !Path::new("net_log").is_dir() {
        // FIXME: handle errors
        std::fs::create_dir("net_log").unwrap();
    }

    let file_name = format!("net_log/net.{log_index:08}.request_{direction}.json");
    let file = std::fs::File::create(&file_name).unwrap();

    let json = serde_json::from_slice(request.body()).ok();

    let headers: SmallVec<[(&str, Cow<str>); 8]> = request
        .headers()
        .iter()
        .map(|(name, value)| (name.as_str(), String::from_utf8_lossy(value.as_bytes())))
        .collect();
    let path = request.uri().to_string();

    let request_entry = RequestEntry {
        path: &path,
        body: String::from_utf8_lossy(request.body()),
        json,
        headers: &headers,
    };

    serde_json::to_writer_pretty(file, &request_entry).unwrap();
}

pub(crate) fn log_network_response(
    log_index: usize,
    response: &Response<Vec<u8>>,
    direction: &str,
) {
    if !Path::new("net_log").is_dir() {
        // FIXME: handle errors
        std::fs::create_dir("net_log").unwrap();
    }

    let file_name = format!("net_log/net.{log_index:08}.response_{direction}.json");
    let file = std::fs::File::create(&file_name).unwrap();

    let json = serde_json::from_slice(response.body()).ok();

    let headers: SmallVec<[(&str, Cow<str>); 8]> = response
        .headers()
        .iter()
        .map(|(name, value)| (name.as_str(), String::from_utf8_lossy(value.as_bytes())))
        .collect();

    let response_entry = ResponseEntry {
        body: String::from_utf8_lossy(response.body()),
        json,
        headers: &headers,
        status_code: response.status().as_u16(),
    };

    serde_json::to_writer_pretty(file, &response_entry).unwrap();
}
