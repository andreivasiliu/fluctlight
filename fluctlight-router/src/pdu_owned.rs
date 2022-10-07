// TODO: This will all be deleted
#![allow(dead_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::{
    matrix_types::{Event, Id, Room, ServerName, User},
    server_keys::{EventHashable, Hashable, Signable, Signatures},
    state::TimeStamp,
};

pub(crate) fn parse_pdu(event: &RawValue) -> Result<PDU<AnyContent>, std::io::Error> {
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

#[derive(Deserialize)]
struct PDUTypeOnly {
    #[serde(rename = "type")]
    pdu_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct SendJoinResponse<'a> {
    #[serde(borrow)]
    pub auth_chain: Vec<&'a RawValue>,
    pub event: PDU<MemberContent>,
    pub state: Vec<&'a RawValue>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct MakeJoinResponse {
    pub event: PDU<MemberContent>,
    pub room_version: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct PDU<Content: PDUContentType> {
    pub auth_events: Vec<Box<Id<Event>>>,
    pub content: Content,
    pub depth: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hashes: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<Box<Id<ServerName>>>,
    pub origin_server_ts: TimeStamp,
    pub prev_events: Vec<Box<Id<Event>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_state: Option<Vec<Box<Id<Event>>>>,
    pub room_id: Box<Id<Room>>,
    pub sender: Box<Id<User>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signatures: Option<Signatures>,
    pub state_key: Content::StateKey,
    #[serde(rename = "type")]
    pub pdu_type: String,
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

impl<C> EventHashable for PDU<C>
where
    C: PDUContentType + Serialize,
    C::StateKey: Serialize,
{
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
    type StateKey = Option<String>;

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
    Other(Option<String>),
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
