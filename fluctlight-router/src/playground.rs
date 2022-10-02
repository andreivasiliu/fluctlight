use std::{collections::BTreeMap, error::Error, io::Write};

use rayon::prelude::ParallelIterator;
use serde::{Deserialize, Serialize};
use serde_json::{json, value::RawValue};

use crate::{
    canonical_hash::verify_content_hash,
    matrix_types::{Event, Id, Room, ServerName, User},
    persistence::{PDUBlob, RoomPersistence},
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

    // FIXME: Use signable trait
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

    Ok(Vec::new())

    /*
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
    */
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

    let make_join_response_bytes = send_signed_request(uri, None, state)?;

    let make_join_response: MakeJoinResponse = serde_json::from_slice(&*make_join_response_bytes)?;

    let mut join_template = make_join_response.event;

    eprintln!("PDU: {:?}", join_template);

    join_template.origin = Some(state.server_name.clone());
    join_template.origin_server_ts = TimeStamp::now();

    join_template.hash();
    join_template.sign(state);
    let event_id = join_template.generate_event_id();

    join_template.verify(state, &state.server_name).unwrap();

    // The spec requires this to be missing when transferred over federation
    // let event_id = join_template.event_id.take().unwrap();

    let bytes = serde_json::to_vec(&join_template)?;

    eprintln!("Template: {}", String::from_utf8_lossy(&bytes));

    let raw_value: &RawValue = serde_json::from_slice(&bytes)?;

    let uri = format!(
        "/_matrix/federation/v2/send_join/{}/{}",
        "!jhTIqlwlxKKoPPHIgH:synapse-dev.demi.ro", event_id,
    );

    let send_join_response_bytes = send_signed_request(uri, Some(raw_value), state)?;

    eprintln!(
        "Join response: {}",
        String::from_utf8_lossy(&send_join_response_bytes)
    );

    let send_join_response: SendJoinResponse = serde_json::from_slice(&send_join_response_bytes)?;

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

pub(crate) fn load_join_event() -> Result<(), Box<dyn Error>> {
    use breezy_timer::{BreezyTimer, Timer};
    let mut timer = BreezyTimer::new();

    timer.start("total");

    timer.start("read file");
    let string = std::fs::read_to_string("matrix_hq.test.json").unwrap();
    timer.stop("read file");

    timer.start("parse JSON");
    let send_join_response: SendJoinResponse = serde_json::from_str(&string)?;
    timer.stop("parse JSON");

    eprintln!("Auth events: {:?}", send_join_response.auth_chain.len());
    eprintln!("State events: {:?}", send_join_response.state.len());

    eprintln!("Parsing events...");
    timer.start("parse events");

    let mut pdus =
        Vec::with_capacity(send_join_response.state.len() + send_join_response.auth_chain.len());

    for &event in send_join_response
        .state
        .iter()
        .chain(send_join_response.auth_chain.iter())
    {
        let parsed_pdu = ParsedPDU {
            event_id: None,
            pdu: parse_pdu(event)?,
            blob: event.to_owned(),
            signature_check: None,
            hash_check: None,
        };

        pdus.push(parsed_pdu);
    }
    timer.stop("parse events");

    eprintln!("Generating event IDs...");
    timer.start("generate event IDs");
    for parsed_pdu in &mut pdus {
        parsed_pdu.event_id = Some(parsed_pdu.pdu.generate_event_id());
    }
    timer.stop("generate event IDs");

    eprintln!("Persisting events on disk...");
    timer.start("persist events");
    std::fs::remove_file("db.room.matrix_hq/state_pdus.json.gz").unwrap();
    let room_path = "db.room.matrix_hq";
    let mut room_persistence =
        RoomPersistence::new(room_path).expect("Could not open room persistence");

    for parsed_pdu in &pdus {
        let pdu_blob = PDUBlob {
            event_id: parsed_pdu.event_id.as_ref().unwrap(),
            pdu_blob: &parsed_pdu.blob,
        };

        serde_json::to_writer(room_persistence.state_pdu_file(), &pdu_blob)
            .expect("Could not write to persistent room storage");
    }
    room_persistence
        .state_pdu_file()
        .flush()
        .expect("Could not flush room persistence store");
    drop(room_persistence);
    timer.stop("persist events");

    eprintln!("Times:");
    for (name, time) in timer {
        println!("{} {:.2}s", name, time.get_total_elapsed().as_secs_f32());
    }

    Ok(())
}

pub(crate) fn load_room(state: &State) -> Result<(), Box<dyn Error>> {
    use breezy_timer::{BreezyTimer, Timer};
    let mut timer = BreezyTimer::new();

    timer.start("total");

    eprintln!("Reloading events from disk...");
    timer.start("reloading events");
    let mut room_persistence = RoomPersistence::new("db.room.matrix_hq").unwrap();

    let pdu_bytes = room_persistence.read_state_pdu_file()?;
    let json_stream = serde_json::Deserializer::from_slice(&pdu_bytes);
    timer.stop("reloading events");

    eprintln!("Reparsing events from disk...");
    timer.start("reparsing events");
    let mut pdus = Vec::new();

    for pdu_blob in json_stream.into_iter::<PDUBlob>() {
        let pdu_blob = pdu_blob.unwrap();
        let parsed_pdu = parse_pdu(&pdu_blob.pdu_blob).unwrap();

        pdus.push(ParsedPDU {
            event_id: Some(pdu_blob.event_id.to_owned()),
            pdu: parsed_pdu,
            blob: pdu_blob.pdu_blob.to_owned(),
            signature_check: None,
            hash_check: None,
        })
    }
    timer.stop("reparsing events");

    eprintln!("Checking event hashes...");
    timer.start("hash events");
    let mut correct = 0;
    let mut incorrect = 0;
    // let mut example = None;
    for parsed_pdu in &mut pdus {
        let json_blob = parsed_pdu.blob.get();
        let result = verify_content_hash(json_blob, false);
        if result.is_err() {
            incorrect += 1;
        } else {
            correct += 1;
        }
        parsed_pdu.hash_check = Some(result);
    }
    eprintln!("Correct: {}, incorrect: {}", correct, incorrect);
    // if let Some(example) = example {
    //     eprintln!("Example: {}", example.get());
    //     eprintln!("Example canonical:");
    //     verify_content_hash(example.get(), true).ok();
    //     eprintln!();
    // }
    timer.stop("hash events");

    eprintln!("Checking event signatures...");
    // let mut missing_keys = BTreeSet::new();

    timer.start("check event signatures");
    use rayon::prelude::ParallelSliceMut;
    pdus.par_chunks_mut(128).for_each(|pdus| {
        for parsed_pdu in pdus {
            // FIXME: No clone
            let sender_name = parsed_pdu.pdu.sender.server_name().to_owned();
            let result = parsed_pdu.pdu.verify(state, &sender_name);
            parsed_pdu.signature_check = Some(result);
        }
    });
    timer.stop("check event signatures");

    eprintln!("Mapping events in memory...");
    timer.start("store events");

    let room_id = pdus[0].pdu.room_id.clone();
    let mut room_pdus: BTreeMap<Box<Id<Event>>, ParsedPDU> = BTreeMap::new();
    let mut room_pdus_by_timestamp: BTreeMap<TimeStamp, Box<Id<Event>>> = BTreeMap::new();

    for parsed_pdu in pdus {
        let event_id = parsed_pdu.event_id.as_ref().expect("Just created").clone();

        room_pdus_by_timestamp.insert(parsed_pdu.pdu.origin_server_ts, event_id.clone());
        room_pdus.insert(event_id, parsed_pdu);
    }

    state.with_ephemeral_mut(|ephemeral| {
        let room = ephemeral.rooms.entry(room_id).or_default();

        room.pdus = room_pdus;
        room.pdus_by_timestamp = room_pdus_by_timestamp;
    });

    timer.stop("store events");

    eprintln!("All done.");

    timer.stop("total");

    eprintln!("Times:");
    for (name, time) in timer {
        println!("{} {:.2}s", name, time.get_total_elapsed().as_secs_f32());
    }

    Ok(())
}

fn parse_pdu(event: &RawValue) -> Result<PDU<AnyContent>, std::io::Error> {
    let pdu: PDUTypeOnly = serde_json::from_str(event.get())?;

    let pdu = match &*pdu.pdu_type {
        "m.room.member" => {
            let pdu: PDU<MemberContent> = serde_json::from_str(event.get())?;
            pdu.upcast()
        }
        "m.room.create" => {
            let pdu: PDU<CreateContent> = serde_json::from_str(event.get())?;
            pdu.upcast()
        }
        "m.room.history_visibility" => {
            let pdu: PDU<HistoryVisibilityContent> = serde_json::from_str(event.get())?;
            pdu.upcast()
        }
        "m.room.join_rules" => {
            let pdu: PDU<JoinRulesContent> = serde_json::from_str(event.get())?;
            pdu.upcast()
        }
        "m.room.power_levels" => {
            let pdu: PDU<PowerLevelsContent> = serde_json::from_str(event.get())?;
            pdu.upcast()
        }
        // FIXME: only on versions 5 and below
        "m.room.aliases" => {
            let pdu: PDU<RoomAliasesContent> = serde_json::from_str(event.get())?;
            pdu.upcast()
        }
        _ => {
            let pdu: PDU<EmptyContent> = serde_json::from_str(event.get())?;
            pdu.upcast()
        }
    };

    Ok(pdu)
}

pub(crate) struct ParsedPDU {
    pub event_id: Option<Box<Id<Event>>>,
    pub pdu: PDU<AnyContent>,
    pub blob: Box<RawValue>,
    pub signature_check: Option<Result<(), &'static str>>,
    pub hash_check: Option<Result<(), String>>,
}

impl ParsedPDU {
    pub(crate) fn blob(&self) -> String {
        self.blob.to_string()
    }
}

#[derive(Deserialize)]
struct PDUTypeOnly {
    #[serde(rename = "type")]
    pdu_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SendJoinResponse<'a> {
    #[serde(borrow)]
    auth_chain: Vec<&'a RawValue>,
    event: PDU<MemberContent>,
    state: Vec<&'a RawValue>,
}

#[derive(Serialize, Deserialize, Debug)]
struct MakeJoinResponse {
    event: PDU<MemberContent>,
    room_version: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct PDU<Content: PDUContentType> {
    pub(crate) auth_events: Vec<Box<Id<Event>>>,
    content: Content,
    depth: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    hashes: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    origin: Option<Box<Id<ServerName>>>,
    origin_server_ts: TimeStamp,
    pub(crate) prev_events: Vec<Box<Id<Event>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    prev_state: Option<Vec<Box<Id<Event>>>>,
    room_id: Box<Id<Room>>,
    sender: Box<Id<User>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    signatures: Option<Signatures>,
    state_key: Content::StateKey,
    #[serde(rename = "type")]
    pub(crate) pdu_type: String,
    // TODO: missing 'membership'
}

impl<C> Signable for PDU<C>
where
    C: PDUContentType + Serialize,
    C::StateKey: Serialize,
{
    fn signatures_mut(&mut self) -> &mut Option<Signatures> {
        &mut self.signatures
    }

    fn signatures(&self) -> &Option<Signatures> {
        &self.signatures
    }
}

impl<C> Hashable for PDU<C>
where
    C: PDUContentType + Serialize,
    C::StateKey: Serialize,
{
    fn hashes_mut(&mut self) -> &mut Option<BTreeMap<String, String>> {
        &mut self.hashes
    }
}

impl<C: PDUContentType> PDU<C> {
    fn upcast(self) -> PDU<AnyContent> {
        PDU {
            auth_events: self.auth_events,
            content: self.content.upcast(),
            depth: self.depth,
            hashes: self.hashes,
            origin: self.origin,
            origin_server_ts: self.origin_server_ts,
            prev_events: self.prev_events,
            prev_state: self.prev_state,
            room_id: self.room_id,
            sender: self.sender,
            signatures: self.signatures,
            state_key: C::upcast_state(self.state_key),
            pdu_type: self.pdu_type,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct MemberContent {
    membership: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct CreateContent {
    creator: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct JoinRulesContent {
    join_rule: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct RoomAliasesContent {
    aliases: RoomAliasesList,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub(crate) enum RoomAliasesList {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
struct PowerLevel(i64);

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct PowerLevelsContent {
    ban: PowerLevel,
    events: BTreeMap<String, PowerLevel>,
    events_default: PowerLevel,
    kick: PowerLevel,
    redact: PowerLevel,
    state_default: PowerLevel,
    users: BTreeMap<Box<Id<User>>, PowerLevel>,
    users_default: PowerLevel,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct HistoryVisibilityContent {
    history_visibility: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct EmptyContent {}

/// The state key for certain event types must always be an empty string.
pub(crate) struct EmptyStateKey;

impl Serialize for EmptyStateKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        "".serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for EmptyStateKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let string = <&str>::deserialize(deserializer)?;

        if !string.is_empty() {
            return Err(serde::de::Error::custom("Expected empty string"));
        }

        Ok(EmptyStateKey)
    }
}

pub(crate) trait PDUContentType {
    type StateKey;

    fn upcast(self) -> AnyContent;
    fn upcast_state(state_key: Self::StateKey) -> AnyState;
}

impl PDUContentType for MemberContent {
    type StateKey = Box<Id<User>>;

    fn upcast(self) -> AnyContent {
        AnyContent::Member(self)
    }
    fn upcast_state(state_key: Self::StateKey) -> AnyState {
        AnyState::UserId(state_key)
    }
}

impl PDUContentType for CreateContent {
    type StateKey = EmptyStateKey;

    fn upcast(self) -> AnyContent {
        AnyContent::Create(self)
    }

    fn upcast_state(state_key: Self::StateKey) -> AnyState {
        AnyState::Empty(state_key)
    }
}

impl PDUContentType for JoinRulesContent {
    type StateKey = EmptyStateKey;

    fn upcast(self) -> AnyContent {
        AnyContent::JoinRules(self)
    }

    fn upcast_state(state_key: Self::StateKey) -> AnyState {
        AnyState::Empty(state_key)
    }
}

impl PDUContentType for PowerLevelsContent {
    type StateKey = EmptyStateKey;

    fn upcast(self) -> AnyContent {
        AnyContent::PowerLevels(self)
    }

    fn upcast_state(state_key: Self::StateKey) -> AnyState {
        AnyState::Empty(state_key)
    }
}

impl PDUContentType for HistoryVisibilityContent {
    type StateKey = EmptyStateKey;

    fn upcast(self) -> AnyContent {
        AnyContent::HistoryVisibility(self)
    }

    fn upcast_state(state_key: Self::StateKey) -> AnyState {
        AnyState::Empty(state_key)
    }
}

impl PDUContentType for RoomAliasesContent {
    type StateKey = Box<Id<ServerName>>;

    fn upcast(self) -> AnyContent {
        AnyContent::RoomAliases(self)
    }

    fn upcast_state(state_key: Self::StateKey) -> AnyState {
        AnyState::ServerName(state_key)
    }
}

impl PDUContentType for EmptyContent {
    type StateKey = String;

    fn upcast(self) -> AnyContent {
        AnyContent::Other(self)
    }

    fn upcast_state(state_key: Self::StateKey) -> AnyState {
        AnyState::Other(state_key)
    }
}

pub(crate) enum AnyContent {
    Member(MemberContent),
    Create(CreateContent),
    JoinRules(JoinRulesContent),
    PowerLevels(PowerLevelsContent),
    HistoryVisibility(HistoryVisibilityContent),
    RoomAliases(RoomAliasesContent),
    Other(EmptyContent),
}

pub(crate) enum AnyState {
    UserId(Box<Id<User>>),
    ServerName(Box<Id<ServerName>>),
    Empty(EmptyStateKey),
    Other(String),
}

impl PDUContentType for AnyContent {
    type StateKey = AnyState;

    fn upcast(self) -> AnyContent {
        self
    }

    fn upcast_state(state_key: Self::StateKey) -> AnyState {
        state_key
    }
}

impl Serialize for AnyContent {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            AnyContent::Member(c) => c.serialize(serializer),
            AnyContent::Create(c) => c.serialize(serializer),
            AnyContent::JoinRules(c) => c.serialize(serializer),
            AnyContent::PowerLevels(c) => c.serialize(serializer),
            AnyContent::HistoryVisibility(c) => c.serialize(serializer),
            AnyContent::RoomAliases(c) => c.serialize(serializer),
            AnyContent::Other(c) => c.serialize(serializer),
        }
    }
}

impl Serialize for AnyState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            AnyState::UserId(s) => s.serialize(serializer),
            AnyState::ServerName(s) => s.serialize(serializer),
            AnyState::Empty(s) => s.serialize(serializer),
            AnyState::Other(s) => s.serialize(serializer),
        }
    }
}
