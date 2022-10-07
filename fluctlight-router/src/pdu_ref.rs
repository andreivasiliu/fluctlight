use std::{borrow::Cow, collections::BTreeMap};

use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;
use smallvec::SmallVec;
use vec_collections::VecMap1;

use crate::{
    matrix_types::{Event, Id, Key, Room, ServerName, User},
    server_keys::{EventHashable, Verifiable},
    state::TimeStamp,
};

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct PDURef<'a, Content: PDUContentType<'a>> {
    #[serde(borrow)]
    pub auth_events: SmallVec<[&'a Id<Event>; 4]>,
    pub content: Content,
    pub depth: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hashes: Option<VecMap1<&'a str, &'a str>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<&'a Id<ServerName>>,
    pub origin_server_ts: TimeStamp,
    pub prev_events: SmallVec<[&'a Id<Event>; 2]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prev_state: Option<Vec<&'a Id<Event>>>,
    pub room_id: &'a Id<Room>,
    #[serde(borrow)]
    pub sender: Cow<'a, Id<User>>,
    // Serialization is done for signing
    #[serde(skip_serializing)]
    pub signatures: Option<SignaturesRef<'a>>,
    #[serde(skip_serializing_if = "Content::missing_state")]
    pub state_key: Content::StateKey,
    #[serde(rename = "type")]
    pub pdu_type: &'a str,
    // TODO: missing 'membership'
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub(crate) struct SignaturesRef<'a> {
    #[serde(borrow)]
    pub signatures: VecMap1<&'a Id<ServerName>, VecMap1<&'a Id<Key>, &'a str>>,
}

impl<'a> SignaturesRef<'a> {
    pub(crate) fn get_signatures<'s>(
        &'s self,
        server_name: &Id<ServerName>,
    ) -> Option<&'s VecMap1<&'a Id<Key>, &'a str>> {
        for (server, signatures) in &self.signatures {
            if *server == server_name {
                return Some(signatures);
            }
        }
        None
    }
}

pub(crate) fn parse_pdu_ref(
    event: &RawValue,
) -> Result<PDURef<'_, AnyContentRef<'_>>, std::io::Error> {
    let pdu: PDUTypeOnly = serde_json::from_str(event.get())?;

    let pdu = match &*pdu.pdu_type {
        "m.room.member" => {
            let pdu: PDURef<MemberContent> = serde_json::from_str(event.get())?;
            pdu.upcast()
        }
        "m.room.create" => {
            let pdu: PDURef<CreateContent> = serde_json::from_str(event.get())?;
            pdu.upcast()
        }
        "m.room.history_visibility" => {
            let pdu: PDURef<HistoryVisibilityContent> = serde_json::from_str(event.get())?;
            pdu.upcast()
        }
        "m.room.join_rules" => {
            let pdu: PDURef<JoinRulesContent> = serde_json::from_str(event.get())?;
            pdu.upcast()
        }
        "m.room.power_levels" => {
            let pdu: PDURef<PowerLevelsContent> = serde_json::from_str(event.get())?;
            pdu.upcast()
        }
        // FIXME: only on versions 5 and below
        "m.room.aliases" => {
            let pdu: PDURef<RoomAliasesContent> = serde_json::from_str(event.get())?;
            pdu.upcast()
        }
        _ => {
            let pdu: PDURef<EmptyContent> = serde_json::from_str(event.get())?;
            pdu.upcast()
        }
    };

    Ok(pdu)
}

#[derive(Deserialize)]
struct PDUTypeOnly<'a> {
    #[serde(rename = "type")]
    pdu_type: &'a str,
}

impl<'a, C> Verifiable for PDURef<'a, C>
where
    C: PDUContentType<'a> + Serialize,
    C::StateKey: Serialize,
{
}

impl<'a, C> EventHashable for PDURef<'a, C>
where
    C: PDUContentType<'a> + Serialize,
    C::StateKey: Serialize,
{
}

impl<'a, C: PDUContentType<'a>> PDURef<'a, C> {
    fn upcast(self) -> PDURef<'a, AnyContentRef<'a>> {
        PDURef {
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
pub(crate) struct MemberContent<'a> {
    pub membership: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct CreateContent<'a> {
    pub creator: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct JoinRulesContent<'a> {
    pub join_rule: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct RoomAliasesContent<'a> {
    #[serde(borrow)]
    pub aliases: RoomAliasesList<'a>,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub(crate) enum RoomAliasesList<'a> {
    Single(&'a str),
    Multiple(Vec<&'a str>),
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[serde(transparent)]
pub(crate) struct PowerLevel(i64);

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct PowerLevelsContent<'a> {
    pub ban: PowerLevel,
    #[serde(borrow)]
    pub events: BTreeMap<&'a str, PowerLevel>,
    pub events_default: PowerLevel,
    pub kick: PowerLevel,
    pub redact: PowerLevel,
    pub state_default: PowerLevel,
    pub users: BTreeMap<&'a Id<User>, PowerLevel>,
    pub users_default: PowerLevel,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct HistoryVisibilityContent<'a> {
    pub history_visibility: &'a str,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct EmptyContent {}

#[derive(Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct UserStateKey<'a> {
    #[serde(borrow)]
    pub user_id: Cow<'a, Id<User>>,
}

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

pub(crate) trait PDUContentType<'a> {
    type StateKey;

    fn upcast(self) -> AnyContentRef<'a>;
    fn upcast_state(state_key: Self::StateKey) -> AnyStateRef<'a>;

    fn has_state(_state_key: &Self::StateKey) -> bool {
        true
    }

    fn missing_state(state_key: &Self::StateKey) -> bool {
        !Self::has_state(state_key)
    }
}

impl<'a> PDUContentType<'a> for MemberContent<'a> {
    type StateKey = UserStateKey<'a>;

    fn upcast(self) -> AnyContentRef<'a> {
        AnyContentRef::Member(self)
    }
    fn upcast_state(state_key: Self::StateKey) -> AnyStateRef<'a> {
        AnyStateRef::UserId(state_key)
    }
}

impl<'a> PDUContentType<'a> for CreateContent<'a> {
    type StateKey = EmptyStateKey;

    fn upcast(self) -> AnyContentRef<'a> {
        AnyContentRef::Create(self)
    }

    fn upcast_state(state_key: Self::StateKey) -> AnyStateRef<'a> {
        AnyStateRef::Empty(state_key)
    }
}

impl<'a> PDUContentType<'a> for JoinRulesContent<'a> {
    type StateKey = EmptyStateKey;

    fn upcast(self) -> AnyContentRef<'a> {
        AnyContentRef::JoinRules(self)
    }

    fn upcast_state(state_key: Self::StateKey) -> AnyStateRef<'a> {
        AnyStateRef::Empty(state_key)
    }
}

impl<'a> PDUContentType<'a> for PowerLevelsContent<'a> {
    type StateKey = EmptyStateKey;

    fn upcast(self) -> AnyContentRef<'a> {
        AnyContentRef::PowerLevels(self)
    }

    fn upcast_state(state_key: Self::StateKey) -> AnyStateRef<'a> {
        AnyStateRef::Empty(state_key)
    }
}

impl<'a> PDUContentType<'a> for HistoryVisibilityContent<'a> {
    type StateKey = EmptyStateKey;

    fn upcast(self) -> AnyContentRef<'a> {
        AnyContentRef::HistoryVisibility(self)
    }

    fn upcast_state(state_key: Self::StateKey) -> AnyStateRef<'a> {
        AnyStateRef::Empty(state_key)
    }
}

impl<'a> PDUContentType<'a> for RoomAliasesContent<'a> {
    type StateKey = &'a Id<ServerName>;

    fn upcast(self) -> AnyContentRef<'a> {
        AnyContentRef::RoomAliases(self)
    }

    fn upcast_state(state_key: Self::StateKey) -> AnyStateRef<'a> {
        AnyStateRef::ServerName(state_key)
    }
}

impl<'a> PDUContentType<'a> for EmptyContent {
    type StateKey = Option<&'a str>;

    fn upcast(self) -> AnyContentRef<'a> {
        AnyContentRef::Other(self)
    }

    fn upcast_state(state_key: Self::StateKey) -> AnyStateRef<'a> {
        AnyStateRef::Other(state_key)
    }

    fn has_state(state_key: &Self::StateKey) -> bool {
        state_key.is_some()
    }
}

pub(crate) enum AnyContentRef<'a> {
    Member(MemberContent<'a>),
    Create(CreateContent<'a>),
    JoinRules(JoinRulesContent<'a>),
    PowerLevels(PowerLevelsContent<'a>),
    HistoryVisibility(HistoryVisibilityContent<'a>),
    RoomAliases(RoomAliasesContent<'a>),
    Other(EmptyContent),
}

pub(crate) enum AnyStateRef<'a> {
    UserId(UserStateKey<'a>),
    ServerName(&'a Id<ServerName>),
    Empty(EmptyStateKey),
    Other(Option<&'a str>),
}

impl<'a> PDUContentType<'a> for AnyContentRef<'a> {
    type StateKey = AnyStateRef<'a>;

    fn upcast(self) -> AnyContentRef<'a> {
        self
    }

    fn upcast_state(state_key: Self::StateKey) -> AnyStateRef<'a> {
        state_key
    }

    fn has_state(state_key: &Self::StateKey) -> bool {
        match state_key {
            AnyStateRef::Other(state) => state.is_some(),
            _ => true,
        }
    }
}

impl<'a> Serialize for AnyContentRef<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            AnyContentRef::Member(c) => c.serialize(serializer),
            AnyContentRef::Create(c) => c.serialize(serializer),
            AnyContentRef::JoinRules(c) => c.serialize(serializer),
            AnyContentRef::PowerLevels(c) => c.serialize(serializer),
            AnyContentRef::HistoryVisibility(c) => c.serialize(serializer),
            AnyContentRef::RoomAliases(c) => c.serialize(serializer),
            AnyContentRef::Other(c) => c.serialize(serializer),
        }
    }
}

impl<'a> Serialize for AnyStateRef<'a> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            AnyStateRef::UserId(s) => s.serialize(serializer),
            AnyStateRef::ServerName(s) => s.serialize(serializer),
            AnyStateRef::Empty(s) => s.serialize(serializer),
            AnyStateRef::Other(s) => s.serialize(serializer),
        }
    }
}
