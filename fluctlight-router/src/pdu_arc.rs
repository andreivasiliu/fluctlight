// TODO: Not sure how this PDU type will be used yet
#![allow(dead_code)]

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use vec_collections::{AbstractVecMap, VecMap1};

use crate::{
    interner::{ArcStr, Interner},
    matrix_types::{Event, Id, Key, Room, ServerName, User},
    pdu_ref::{AnyContentRef, AnyStateRef, PDURef, PowerLevel},
    state::TimeStamp,
};

pub(crate) struct PDUArc {
    pub auth_events: SmallVec<[ArcStr<Id<Event>>; 4]>,
    pub content: AnyContent,
    pub depth: u64,
    pub hashes: Option<VecMap1<ArcStr<str>, String>>,
    pub origin: Option<ArcStr<Id<ServerName>>>,
    pub origin_server_ts: TimeStamp,
    pub prev_events: SmallVec<[ArcStr<Id<Event>>; 2]>,
    pub prev_state: Option<Vec<ArcStr<Id<Event>>>>,
    pub room_id: ArcStr<Id<Room>>,
    pub sender: ArcStr<Id<User>>,
    pub signatures: Option<Signatures>,
    pub state_key: AnyState,
    pub pdu_type: ArcStr<str>,
    // TODO: missing 'membership'
}

impl PDUArc {
    pub(crate) fn from_pdu_ref<'a>(pdu_ref: &'_ PDURef<'a, AnyContentRef<'a>>, interner: &mut Interner) -> Self {
        PDUArc {
            auth_events: pdu_ref
                .auth_events
                .iter()
                .map(|&event| interner.get_or_insert(event))
                .collect(),
            content: AnyContent::from_ref(&pdu_ref.content, interner),
            depth: pdu_ref.depth,
            hashes: pdu_ref.hashes.as_ref().map(|hashes| {
                hashes
                    .iter()
                    .map(|(key, value)| (interner.get_or_insert(*key), value.to_string()))
                    .collect()
            }),
            origin: pdu_ref.origin.map(|origin| interner.get_or_insert(origin)),
            origin_server_ts: pdu_ref.origin_server_ts,
            prev_events: pdu_ref
                .prev_events
                .iter()
                .map(|&event| interner.get_or_insert(event))
                .collect(),
            prev_state: pdu_ref.prev_state.as_ref().map(|prev_state| {
                prev_state
                    .iter()
                    .map(|&event| interner.get_or_insert(event))
                    .collect()
            }),
            room_id: interner.get_or_insert(&pdu_ref.room_id),
            sender: interner.get_or_insert(&pdu_ref.sender),
            signatures: pdu_ref.signatures.as_ref().map(|signatures| Signatures {
                signatures: signatures
                    .signatures
                    .iter()
                    .map(|(server_name, signatures)| {
                        (
                            interner.get_or_insert(*server_name),
                            signatures
                                .iter()
                                .map(|(key, signature)| {
                                    (interner.get_or_insert(*key), signature.to_string())
                                })
                                .collect(),
                        )
                    })
                    .collect(),
            }),
            state_key: AnyState::from_ref(&pdu_ref.state_key, interner),
            pdu_type: interner.get_or_insert(&pdu_ref.pdu_type),
        }
    }
}

pub(crate) struct Signatures {
    pub signatures: VecMap1<ArcStr<Id<ServerName>>, VecMap1<ArcStr<Id<Key>>, String>>,
}

pub(crate) struct MemberContent {
    membership: ArcStr<str>,
}

pub(crate) struct CreateContent {
    creator: ArcStr<str>,
}

pub(crate) struct JoinRulesContent {
    join_rule: ArcStr<str>,
}

pub(crate) struct RoomAliasesContent {
    aliases: RoomAliasesList,
}

pub(crate) enum RoomAliasesList {
    Single(ArcStr<str>),
    Multiple(Vec<ArcStr<str>>),
}

pub(crate) struct PowerLevelsContent {
    ban: PowerLevel,
    events: BTreeMap<ArcStr<str>, PowerLevel>,
    events_default: PowerLevel,
    kick: PowerLevel,
    redact: PowerLevel,
    state_default: PowerLevel,
    users: BTreeMap<ArcStr<Id<User>>, PowerLevel>,
    users_default: PowerLevel,
}

pub(crate) struct HistoryVisibilityContent {
    history_visibility: ArcStr<str>,
}

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

pub(crate) enum AnyContent {
    Member(MemberContent),
    Create(CreateContent),
    JoinRules(JoinRulesContent),
    PowerLevels(PowerLevelsContent),
    HistoryVisibility(HistoryVisibilityContent),
    RoomAliases(RoomAliasesContent),
    Other(EmptyContent),
}

impl AnyContent {
    fn from_ref(content_ref: &AnyContentRef, interner: &mut Interner) -> Self {
        match content_ref {
            AnyContentRef::Member(crate::pdu_ref::MemberContent { membership }) => {
                AnyContent::Member(MemberContent {
                    membership: interner.get_or_insert(membership),
                })
            }
            AnyContentRef::Create(crate::pdu_ref::CreateContent { creator }) => {
                AnyContent::Create(CreateContent {
                    creator: interner.get_or_insert(creator),
                })
            }
            AnyContentRef::JoinRules(crate::pdu_ref::JoinRulesContent { join_rule }) => {
                AnyContent::JoinRules(JoinRulesContent {
                    join_rule: interner.get_or_insert(join_rule),
                })
            }
            AnyContentRef::PowerLevels(crate::pdu_ref::PowerLevelsContent {
                ban,
                events,
                events_default,
                kick,
                redact,
                state_default,
                users,
                users_default,
            }) => AnyContent::PowerLevels(PowerLevelsContent {
                ban: *ban,
                events: events
                    .iter()
                    .map(|(&event, &power_level)| (interner.get_or_insert(event), power_level))
                    .collect(),
                events_default: *events_default,
                kick: *kick,
                redact: *redact,
                state_default: *state_default,
                users: users
                    .iter()
                    .map(|(&user, &power_level)| (interner.get_or_insert(user), power_level))
                    .collect(),
                users_default: *users_default,
            }),
            AnyContentRef::HistoryVisibility(crate::pdu_ref::HistoryVisibilityContent {
                history_visibility,
            }) => AnyContent::HistoryVisibility(HistoryVisibilityContent {
                history_visibility: interner.get_or_insert(history_visibility),
            }),
            AnyContentRef::RoomAliases(crate::pdu_ref::RoomAliasesContent {
                aliases: crate::pdu_ref::RoomAliasesList::Single(alias),
            }) => AnyContent::RoomAliases(RoomAliasesContent {
                aliases: RoomAliasesList::Single(interner.get_or_insert(alias)),
            }),
            AnyContentRef::RoomAliases(crate::pdu_ref::RoomAliasesContent {
                aliases: crate::pdu_ref::RoomAliasesList::Multiple(aliases),
            }) => AnyContent::RoomAliases(RoomAliasesContent {
                aliases: RoomAliasesList::Multiple(
                    aliases
                        .iter()
                        .map(|&alias| interner.get_or_insert(alias))
                        .collect(),
                ),
            }),
            AnyContentRef::Other(crate::pdu_ref::EmptyContent {}) => {
                AnyContent::Other(EmptyContent {})
            }
        }
    }
}

pub(crate) enum AnyState {
    UserId(ArcStr<Id<User>>),
    ServerName(ArcStr<Id<ServerName>>),
    Empty(EmptyStateKey),
    Other(String),
}

impl AnyState {
    fn from_ref(state_ref: &AnyStateRef, interner: &mut Interner) -> Self {
        match state_ref {
            AnyStateRef::UserId(user_id) => AnyState::UserId(interner.get_or_insert(&user_id.user_id)),
            AnyStateRef::ServerName(server_name) => {
                AnyState::ServerName(interner.get_or_insert(server_name))
            }
            AnyStateRef::Empty(crate::pdu_ref::EmptyStateKey) => AnyState::Empty(EmptyStateKey),
            AnyStateRef::Other(other) => AnyState::Other(other.to_string()),
        }
    }
}
