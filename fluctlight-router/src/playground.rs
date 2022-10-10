use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    io::Write,
};

use rayon::prelude::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use serde_json::{json, value::RawValue, Value};
use vec_collections::VecMap1;

use crate::{
    canonical_hash::verify_content_hash,
    edu_ref::parse_edu_ref,
    interner::{ArcStr, Interner},
    matrix_types::{Event, Id, Key, Room},
    pdu_arc::{AnyContent, PDUArc},
    pdu_owned::{parse_pdu, MakeJoinResponse, SendJoinResponse},
    pdu_ref::{parse_pdu_ref, AnyContentRef, PDURef},
    persistence::{PDUBlob, RoomPersistence},
    server_keys::{EventHashable, Hashable, Signable, Verifiable},
    signed_request::SignedRequestBuilder,
    state::{State, TimeStamp},
};

pub(crate) struct ParsedPDU {
    pub event_id: Box<Id<Event>>,
    pub arc_event_id: Option<ArcStr<Id<Event>>>,
    pub pdu: PDUArc,
    pub blob: Box<RawValue>,
    pub signature_check: Option<Result<(), &'static str>>,
    pub hash_check: Option<Result<(), String>>,
}

impl ParsedPDU {
    pub(crate) fn render_contents(&self) -> String {
        match &self.pdu.content {
            AnyContent::Member(member) => {
                format!("{} {}", self.pdu.sender, member.membership)
            }
            AnyContent::Create(create) => {
                format!("{} created the room", create.creator)
            }
            AnyContent::JoinRules(rules) => {
                format!(
                    "{} changed join rules to {}",
                    self.pdu.sender, rules.join_rule
                )
            }
            AnyContent::PowerLevels(_) => {
                format!("{} changed power levels", self.pdu.sender)
            }
            AnyContent::HistoryVisibility(visibility) => {
                format!(
                    "{} made room {}",
                    self.pdu.sender, visibility.history_visibility
                )
            }
            AnyContent::RoomAliases(_) => {
                format!("{} set room aliases", self.pdu.sender)
            }
            AnyContent::Other(_) => match &*self.pdu.pdu_type {
                "m.room.message" => {
                    let message_pdu: MessagePDU = serde_json::from_str(self.blob.get()).unwrap();
                    format!("{} says \"{}\"", self.pdu.sender, message_pdu.content.body)
                }
                _ => format!("{} unknown", self.pdu.sender),
            },
        }
    }

    pub(crate) fn render_pdu(&self) -> String {
        let value: Value = serde_json::from_str(self.blob.get()).unwrap();
        serde_json::to_string_pretty(&value).unwrap()
    }
}

#[derive(Deserialize)]
struct MessagePDU<'a> {
    #[serde(borrow)]
    content: MessageContent<'a>,
}

#[derive(Deserialize)]
struct MessageContent<'a> {
    body: &'a str,
}

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

    let mut server_signatures: VecMap1<Box<Id<Key>>, String> = VecMap1::empty();

    for (key_name, server_key) in &state.server_key_pairs {
        let noise = None;
        let signature = server_key.key_pair.sk.sign(&signable_bytes, noise);
        let sig_b64 = base64::encode_config(&*signature, base64::STANDARD_NO_PAD);

        server_signatures.insert(key_name.clone(), sig_b64);
    }

    Ok(Vec::new())

    // let bytes = ureq::post(uri)
    //     .set("X-My-Header", "Secret")
    //     .send_json(json!({
    //         "name": "martin",
    //         "rust": true
    //     }))?
    //     .into_string()?
    //     .into_bytes();

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

#[derive(Deserialize)]
struct BackfillResponse {
    pdus: Vec<Box<RawValue>>,
}

pub(crate) fn send_backfill_request(state: &State) -> Result<(), Box<dyn Error>> {
    let room_id = Id::<Room>::try_from_str("!jhTIqlwlxKKoPPHIgH:synapse-dev.demi.ro").unwrap();
    let event_id = "$By7ZDI3wONJDXly1um6f1NqimBdqS_1g3kxeNYjhnBA";

    let uri = format!("/_matrix/federation/v1/backfill/{room_id}?limit=50&v={event_id}");
    let response: BackfillResponse = SignedRequestBuilder::get(state, &uri)
        .destination("synapse-dev.demi.ro")
        .send()?;

    eprintln!("Got {} backfill PDUs.", response.pdus.len());
    state.with_persistent_mut(|persistent_state| {
        let room = persistent_state.rooms.get_mut(room_id).unwrap();

        for pdu in response.pdus {
            room.pdu_blobs.push(pdu.to_string());
        }
    });

    Ok(())
}

pub(crate) fn send_join_request(state: &State) -> Result<(), Box<dyn Error>> {
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

    // FIXME
    // join_template.verify(state, &state.server_name, signatures).unwrap();

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

    let mut interner = Interner::new();

    eprintln!("Parsing events...");
    timer.start("parse events");

    let mut pdus =
        Vec::with_capacity(send_join_response.state.len() + send_join_response.auth_chain.len());

    for &event in send_join_response
        .state
        .iter()
        .chain(send_join_response.auth_chain.iter())
    {
        // let pdu = parse_pdu(event).unwrap();
        let pdu_ref = parse_pdu_ref(event)?;
        let event_id = pdu_ref.generate_event_id();
        let pdu_arc = PDUArc::from_pdu_ref(&pdu_ref, &mut interner);
        let parsed_pdu = ParsedPDU {
            event_id: event_id,
            arc_event_id: None,
            pdu: pdu_arc,
            blob: event.to_owned(),
            signature_check: None,
            hash_check: None,
        };

        pdus.push(parsed_pdu);
    }
    timer.stop("parse events");

    eprintln!("Persisting events on disk...");
    timer.start("persist events");
    std::fs::remove_file("db.room.matrix_hq/state_pdus.json.gz").unwrap();
    let room_path = "db.room.matrix_hq";
    let mut room_persistence =
        RoomPersistence::new(room_path).expect("Could not open room persistence");

    for parsed_pdu in &pdus {
        let pdu_blob = PDUBlob {
            event_id: &parsed_pdu.event_id,
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
    timer.stop("reloading events");

    eprintln!("Reparsing events from disk...");
    timer.start("reparsing events");
    let mut pdu_blobs = Vec::new();
    let json_stream = serde_json::Deserializer::from_slice(&pdu_bytes);

    for pdu_blob in json_stream.into_iter::<PDUBlob>() {
        let pdu_blob = pdu_blob.unwrap();
        pdu_blobs.push(pdu_blob);
    }

    let mut interner = Interner::new();

    struct PartialPDU<'a> {
        pdu_ref: PDURef<'a, AnyContentRef<'a>>,
        signature_check: Result<(), &'static str>,
        event_id: Box<Id<Event>>,
        pdu_blob: Box<RawValue>,
    }
    println!(
        "Allocated before parsing: {}MB",
        crate::ALLOCATOR.allocated() / 1024 / 1024
    );

    let partial_pdus: Vec<_> = pdu_blobs
        .into_par_iter()
        .map(|pdu_blob| {
            let pdu_ref = parse_pdu_ref(&pdu_blob.pdu_blob).unwrap();

            let signatures = pdu_ref.signatures.as_ref().unwrap();
            let sender_name = pdu_ref.sender.server_name();

            let signature_check = pdu_ref.verify(state, sender_name, signatures);

            // if let AnyStateRef::UserId(UserStateKey { user_id }) = &pdu_ref.state_key {
            //     if let std::borrow::Cow::Owned(user_id) = &user_id {
            //         panic!("{user_id} was owned");
            //     }
            // }

            // let empty_blob: Box<RawValue> = serde_json::from_str("42").unwrap();

            PartialPDU {
                pdu_ref,
                signature_check,
                pdu_blob: pdu_blob.pdu_blob.to_owned(),
                event_id: pdu_blob.event_id.to_owned(),
            }
        })
        .collect();

    println!(
        "Allocated after parsing: {}MB",
        crate::ALLOCATOR.allocated() / 1024 / 1024
    );
    let mut pdus = Vec::with_capacity(partial_pdus.len());

    println!(
        "Added {} ref PDUs of size {}, total: {}",
        partial_pdus.len(),
        std::mem::size_of::<crate::pdu_ref::PDURef<crate::pdu_ref::AnyContentRef>>(),
        std::mem::size_of::<crate::pdu_ref::PDURef<crate::pdu_ref::AnyContentRef>>()
            * partial_pdus.len(),
    );

    for partial_pdu in partial_pdus {
        let pdu_arc = PDUArc::from_pdu_ref(&partial_pdu.pdu_ref, &mut interner);

        pdus.push(ParsedPDU {
            event_id: partial_pdu.event_id,
            arc_event_id: None,
            pdu: pdu_arc,
            blob: partial_pdu.pdu_blob,
            signature_check: Some(partial_pdu.signature_check),
            hash_check: None,
        });
    }

    println!(
        "Allocated after interning: {}MB",
        crate::ALLOCATOR.allocated() / 1024 / 1024
    );

    drop(pdu_bytes);
    println!(
        "Allocated after dropping bytes: {}MB",
        crate::ALLOCATOR.allocated() / 1024 / 1024
    );

    interner.print_memory_usage();

    timer.stop("reparsing events");

    println!(
        "Added {} arc PDUs of size {}, total: {}",
        pdus.len(),
        std::mem::size_of::<crate::pdu_arc::PDUArc>(),
        std::mem::size_of::<crate::pdu_arc::PDUArc>() * pdus.len(),
    );

    eprintln!("Checking event hashes...");
    timer.start("hash events");
    let mut correct = 0;
    let mut incorrect = 0;
    // let mut example = None;
    for parsed_pdu in pdus.iter_mut() {
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
    println!(
        "Allocated after hashing: {}MB",
        crate::ALLOCATOR.allocated() / 1024 / 1024
    );

    eprintln!("Mapping events in memory...");
    timer.start("store events");

    let room_id = pdus[0].pdu.room_id.to_box();
    let mut room_pdus: BTreeMap<ArcStr<Id<Event>>, ParsedPDU> = BTreeMap::new();
    let mut room_pdus_by_timestamp: BTreeMap<TimeStamp, ArcStr<Id<Event>>> = BTreeMap::new();

    for parsed_pdu in pdus {
        let event_id = &parsed_pdu.event_id;

        let event_id = interner.get_or_insert(event_id.as_id());

        room_pdus_by_timestamp.insert(parsed_pdu.pdu.origin_server_ts, event_id.clone());
        room_pdus.insert(event_id, parsed_pdu);
    }

    state.with_ephemeral_mut(|ephemeral| {
        let room = ephemeral.rooms.entry(room_id).or_default();

        room.pdus = room_pdus;
        room.pdus_by_timestamp = room_pdus_by_timestamp;
        room.interner = interner;
    });
    println!(
        "Allocated after storing: {}MB",
        crate::ALLOCATOR.allocated() / 1024 / 1024
    );

    timer.stop("store events");

    eprintln!("All done.");

    timer.stop("total");

    eprintln!("Times:");
    for (name, time) in timer {
        println!("{} {:.2}s", name, time.get_total_elapsed().as_secs_f32());
    }

    Ok(())
}

pub(crate) fn load_persistent_rooms(state: &crate::state::State) {
    let mut rooms = BTreeSet::new();

    state.with_persistent_mut(|persistent_state| {
        for room_id in persistent_state.rooms.keys() {
            rooms.insert(room_id.to_box());
        }
    });

    for room_id in rooms {
        eprintln!("Loading persistent room: {room_id}");
        load_persistent_room(state, &room_id);
    }
}

pub(crate) fn load_persistent_room(state: &crate::state::State, room_id: &Id<Room>) {
    let mut pdu_blobs = Vec::new();

    state.with_persistent_mut(|persistent_state| {
        let room = persistent_state.rooms.get(room_id).unwrap();

        for pdu_blob in &room.pdu_blobs {
            let pdu_blob: Box<RawValue> = serde_json::from_str(pdu_blob).unwrap();
            pdu_blobs.push(pdu_blob);
        }
    });

    state.with_ephemeral_mut(|ephemeral_state| {
        for pdu_blob in pdu_blobs {
            let pdu_ref = parse_pdu_ref(&pdu_blob).unwrap();
            let event_id = pdu_ref.generate_event_id();

            let signatures = pdu_ref.signatures.as_ref().unwrap();
            let sender_name = pdu_ref.sender.server_name();

            let signature_check = pdu_ref.verify(state, sender_name, signatures);
            let hash_check = verify_content_hash(pdu_blob.get(), false);

            let room = ephemeral_state
                .rooms
                .entry(pdu_ref.room_id.to_owned())
                .or_default();
            let interner = &mut room.interner;

            let pdu_arc = PDUArc::from_pdu_ref(&pdu_ref, interner);
            let arc_event_id = interner.get_or_insert(event_id.as_id());
            drop(pdu_ref);
            let timestamp = pdu_arc.origin_server_ts;

            let parsed_pdu = ParsedPDU {
                event_id,
                arc_event_id: Some(arc_event_id.clone()),
                pdu: pdu_arc,
                blob: pdu_blob,
                signature_check: Some(signature_check),
                hash_check: Some(hash_check),
            };

            room.pdus_by_timestamp
                .insert(timestamp, arc_event_id.clone());
            room.pdus.insert(arc_event_id, parsed_pdu);
        }
    });
}

pub(crate) fn ingest_transaction(
    state: &crate::state::State,
    transaction_id: &str,
    origin: &str,
    origin_server_ts: TimeStamp,
    pdus: Vec<&RawValue>,
    edus: Vec<&RawValue>,
) -> BTreeMap<Box<Id<Event>>, Result<(), String>> {
    let mut parsed_pdus = Vec::new();

    let origin = Id::try_from_str(origin).unwrap();

    let time = origin_server_ts.as_millis();

    eprintln!("Transaction {transaction_id} from {origin} at {time}:");

    for edu in edus {
        match parse_edu_ref(edu) {
            Ok(edu_ref) => {
                eprintln!("* Parsed EDU: {}", edu_ref);
            }
            Err(err) => {
                eprintln!("* Got EDU: {}", edu);
                eprintln!("* Error parsing EDU: {}", err);
            }
        }
    }

    for pdu in &pdus {
        eprintln!("* Got PDU: {}", pdu);

        match parse_pdu_ref(pdu) {
            Ok(pdu_ref) => {
                parsed_pdus.push((pdu_ref, *pdu));
            }
            Err(err) => {
                eprintln!("* Error parsing PDU: {}", err);
            }
        }
    }

    let mut pdu_results = BTreeMap::new();

    state.with_persistent_mut(|persistent_state| {
        for (pdu_ref, pdu_blob) in &parsed_pdus {
            if pdu_ref.room_id.as_str() == "!jhTIqlwlxKKoPPHIgH:synapse-dev.demi.ro" {
                let pdu = parse_pdu(pdu_blob);

                match pdu {
                    Ok(mut pdu) => {
                        pdu.origin = Some(origin.to_box());
                        let event_id = pdu.generate_event_id();
                        eprintln!("* Got persistent super-public room PDU: {event_id}");
                        let room = persistent_state.rooms.get_mut(pdu.room_id.as_id()).unwrap();
                        room.pdu_blobs.push(pdu_blob.get().to_owned());
                    }
                    Err(err) => {
                        eprintln!("* Got error in super-public PDU: {err}");
                    }
                }
            }
        }
    });

    state.with_ephemeral_mut(|ephemeral_state| {
        for (mut pdu_ref, pdu_blob) in parsed_pdus {
            pdu_ref.origin = Some(origin);
            let server_name = pdu_ref.sender.server_name();
            let signatures = pdu_ref.signatures.as_ref().unwrap();
            let signature_check = pdu_ref.verify(state, server_name, signatures);
            let hash_check = verify_content_hash(pdu_blob.get(), false);

            if let Some(room) = ephemeral_state.rooms.get_mut(pdu_ref.room_id) {
                let interner = &mut room.interner;

                let event_id = pdu_ref.generate_event_id();
                let arc_event_id = interner.get_or_insert(&*event_id);
                let pdu_arc = PDUArc::from_pdu_ref(&pdu_ref, interner);

                if let Err(_err) = &signature_check {
                    eprintln!("* Warning: Signature check failed: {event_id}");
                }

                if let Err(_err) = &hash_check {
                    eprintln!("* Warning: Hash check failed: {event_id}");
                }

                let parsed_pdu = ParsedPDU {
                    event_id: event_id.clone(),
                    arc_event_id: Some(arc_event_id.clone()),
                    pdu: pdu_arc,
                    blob: pdu_blob.to_owned(),
                    signature_check: Some(signature_check),
                    hash_check: Some(hash_check),
                };

                room.pdus.insert(arc_event_id, parsed_pdu);
                pdu_results.insert(event_id, Ok(()));
            }
        }
    });

    pdu_results
}
