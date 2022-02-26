use std::{collections::BTreeMap, error::Error};

use serde::{Deserialize, Serialize};
use serde_json::{json, value::RawValue};

use crate::{
    matrix_types::{Event, Id, Room, ServerName, User},
    server_keys::{Hashable, Signable, Signatures},
    state::{State, TimeStamp},
};

fn send_signed_request(
    uri: String,
    content: Option<&RawValue>,
    state: &State,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let json = if let Some(content) = content {
        json!({
            "content": content,
            "destination": "synapse-dev.demi.ro",
            "method": "PUT",
            "origin": "fluctlight-dev.demi.ro",
            "uri": uri,
        })
    } else {
        json!({
            "destination": "synapse-dev.demi.ro",
            "method": "GET",
            "origin": "fluctlight-dev.demi.ro",
            "uri": uri,
        })
    };

    let signable_bytes = serde_json::to_vec(&json).unwrap();

    eprintln!(
        "Signing request: \n---\n{}\n---\n",
        String::from_utf8_lossy(&signable_bytes)
    );

    let mut server_signatures = BTreeMap::new();

    for (key_name, server_key) in &state.server_key_pairs {
        let noise = None;
        let signature = server_key.key_pair.sk.sign(&signable_bytes, noise);
        let sig_b64 = base64::encode_config(&*signature, base64::STANDARD_NO_PAD);

        server_signatures.insert(key_name.clone(), sig_b64);
    }

    let client = reqwest::blocking::Client::new();

    let url = format!("https://synapse-dev.demi.ro{}", uri);
    let mut request = if let Some(content) = content {
        client.put(url).body(content.get().as_bytes().to_vec())
    } else {
        client.get(url)
    };

    for (key_name, signature) in server_signatures {
        request = request.header(
            "Authorization",
            format!(
                "X-Matrix origin={},key=\"{}\",sig=\"{}\"",
                state.server_name, key_name, signature
            ),
        );
    }

    eprintln!("Request: {:?}", request);

    let response = request.send();

    eprintln!("Response: {:?}", response);

    let bytes = response?.bytes().unwrap();

    eprintln!("Response bytes: {}", String::from_utf8_lossy(&*bytes));

    Ok(bytes.to_vec())
}

pub(crate) fn send_request(state: &State) -> Result<(), Box<dyn Error>> {
    // let uri = format!(
    //     "/_matrix/federation/v1/state/{}?eventId={}",
    //     "!jhTIqlwlxKKoPPHIgH:synapse-dev.demi.ro",
    //     "$keEA7MhamQCs-qEiXbtfYJtyWFZysXXoZqRMLKrp3ps",
    // );
    let uri = format!(
        "/_matrix/federation/v1/make_join/{}/{}?ver=6",
        "!jhTIqlwlxKKoPPHIgH:synapse-dev.demi.ro", "@whyte:fluctlight-dev.demi.ro",
    );

    let bytes = send_signed_request(uri, None, state)?;

    let make_join_response: MakeJoinResponse = serde_json::from_slice(&*bytes)?;

    let mut join_template = make_join_response.event;

    eprintln!("PDU: {:?}", join_template);

    join_template.origin = state.server_name.clone();
    join_template.origin_server_ts = TimeStamp::now();

    join_template.hash();
    join_template.sign(state);

    join_template.verify(state, &state.server_name);

    // The spec requires this to be missing when transferred over federation
    let event_id = join_template.event_id.take().unwrap();

    let bytes = serde_json::to_vec(&join_template)?;

    eprintln!("Template: {}", String::from_utf8_lossy(&bytes));

    let raw_value: &RawValue = serde_json::from_slice(&bytes)?;

    let uri = format!(
        "/_matrix/federation/v2/send_join/{}/{}",
        "!jhTIqlwlxKKoPPHIgH:synapse-dev.demi.ro", event_id,
    );

    let join_response_bytes = send_signed_request(uri, Some(raw_value), state)?;

    eprintln!(
        "Join response: {}",
        String::from_utf8_lossy(&join_response_bytes)
    );

    let send_join_response: SendJoinResponse = serde_json::from_slice(&join_response_bytes)?;

    eprintln!("Join event: {:?}", send_join_response.event.event_id);
    eprintln!("Auth events: {:?}", send_join_response.auth_chain.len());
    eprintln!("State events: {:?}", send_join_response.state.len());

    state.with_persistent_mut(move |persistent| {
        let send_join_response = send_join_response;
        let room = persistent
            .rooms
            .entry(send_join_response.event.room_id)
            .or_default();
        for event in send_join_response.state.iter().rev() {
            room.pdu_blobs.push(event.to_string());
        }
    });

    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
struct SendJoinResponse<'a> {
    #[serde(borrow)]
    auth_chain: Vec<&'a RawValue>,
    event: JoinPDU,
    state: Vec<&'a RawValue>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MakeJoinResponse {
    event: JoinPDU,
    room_version: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct PDU<Content, StateKey> {
    auth_events: Vec<Box<Id<Event>>>,
    content: Content,
    depth: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    event_id: Option<Box<Id<Event>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    hashes: Option<BTreeMap<String, String>>,
    origin: Box<Id<ServerName>>,
    origin_server_ts: TimeStamp,
    prev_events: Vec<Box<Id<Event>>>,
    prev_state: Vec<Box<Id<Event>>>,
    room_id: Box<Id<Room>>,
    sender: Box<Id<User>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    signatures: Option<Signatures>,
    state_key: StateKey,
    #[serde(rename = "type")]
    pdu_type: String,
}

impl<C: Serialize, S: Serialize> Signable for PDU<C, S> {
    fn signatures_mut(&mut self) -> &mut Option<Signatures> {
        &mut self.signatures
    }

    fn take_event_id(&mut self) -> Option<Box<Id<Event>>> {
        self.event_id.take()
    }

    fn put_event_id(&mut self, event_id: Option<Box<Id<Event>>>) {
        self.event_id = event_id;
    }
}

impl<C: Serialize, S: Serialize> Hashable for PDU<C, S> {
    fn event_id_mut(&mut self) -> &mut Option<Box<Id<Event>>> {
        &mut self.event_id
    }

    fn hashes_mut(&mut self) -> &mut Option<BTreeMap<String, String>> {
        &mut self.hashes
    }
}

type JoinPDU = PDU<JoinContent, Box<Id<User>>>;

#[derive(Serialize, Deserialize, Debug)]
struct JoinContent {
    membership: String,
}
