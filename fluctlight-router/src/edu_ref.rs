use std::fmt::Display;

use serde::Deserialize;
use serde_json::value::RawValue;
use smallvec::SmallVec;
use vec_collections::VecMap1;

use crate::matrix_types::{Id, Room, User, Event};

#[derive(Deserialize)]
struct EDUTypeOnly<'a> {
    edu_type: &'a str,
}

#[derive(Deserialize)]
pub(crate) struct EDURef<'a, C: EDUContentRef<'a>> {
    content: C,
    edu_type: &'a str,
}

pub(crate) fn parse_edu_ref(event: &RawValue) -> Result<EDURef<'_, AnyEDUContentRef<'_>>, std::io::Error> {
    let edu: EDUTypeOnly = serde_json::from_str(event.get())?;

    let edu = match edu.edu_type {
        "m.presence" => {
            let edu: EDURef<PresenceRef> = serde_json::from_str(event.get())?;
            edu.upcast()
        }
        "m.typing" => {
            let edu: EDURef<TypingRef> = serde_json::from_str(event.get())?;
            edu.upcast()
        }
        "m.receipt" => {
            let edu: EDURef<ReceiptRef> = serde_json::from_str(event.get())?;
            edu.upcast()
        }
        _ => {
            let edu: EDURef<UnknownContent> = serde_json::from_str(event.get())?;
            edu.upcast()
        }
    };

    Ok(edu)
}

impl<'a, C: EDUContentRef<'a>> EDURef<'a, C> {
    fn upcast(self) -> EDURef<'a, AnyEDUContentRef<'a>> {
        EDURef {
            content: self.content.upcast(),
            edu_type: self.edu_type,
        }
    }
}

pub(crate) trait EDUContentRef<'a> {
    fn upcast(self) -> AnyEDUContentRef<'a>;
}

pub(crate) enum AnyEDUContentRef<'a> {
    Typing(TypingRef<'a>),
    Presence(PresenceRef<'a>),
    Receipt(ReceiptRef<'a>),
    Unknown(UnknownContent),
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub(crate) struct TypingRef<'a> {
    #[serde(borrow)]
    room_id: &'a Id<Room>,
    typing: bool,
    user_id: &'a Id<User>,
}

#[derive(Deserialize)]
pub(crate) struct PresenceRef<'a> {
    #[serde(borrow)]
    push: SmallVec<[UserPresenceUpdateRef<'a>; 1]>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct UserPresenceUpdateRef<'a> {
    currently_active: Option<bool>,
    last_active_ago: u32,
    presence: PresenceStatus,
    status_msg: Option<&'a str>,
    user_id: &'a Id<User>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum PresenceStatus {
    Offline,
    Unavailable,
    Online,
}

impl Display for PresenceStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PresenceStatus::Offline => "offline",
            PresenceStatus::Unavailable => "unavailable",
            PresenceStatus::Online => "online",
        }.fmt(f)
    }
}

#[derive(Deserialize)]
#[repr(transparent)]
pub(crate) struct ReceiptRef<'a> {
    #[serde(borrow)]
    room_receipts: VecMap1<&'a Id<Room>, RoomReceipt<'a>>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct RoomReceipt<'a> {
    #[serde(borrow)]
    #[serde(rename = "m.read")]
    m_read: UserReadReceipt<'a>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct UserReadReceipt<'a> {
    #[serde(borrow)]
    data: ReadReceiptMetadata<'a>,
    event_ids: SmallVec<[&'a Id<Event>; 1]>,
}

#[allow(dead_code)]
#[derive(Deserialize)]
struct ReadReceiptMetadata<'a> {
    thread_id: Option<&'a str>,
    ts: u64,
}

#[derive(Deserialize)]
pub(crate) struct UnknownContent { }

impl<'a> EDUContentRef<'a> for TypingRef<'a> {
    fn upcast(self) -> AnyEDUContentRef<'a> {
        AnyEDUContentRef::Typing(self)
    }
}

impl<'a> EDUContentRef<'a> for PresenceRef<'a> {
    fn upcast(self) -> AnyEDUContentRef<'a> {
        AnyEDUContentRef::Presence(self)
    }
}

impl<'a> EDUContentRef<'a> for ReceiptRef<'a> {
    fn upcast(self) -> AnyEDUContentRef<'a> {
        AnyEDUContentRef::Receipt(self)
    }
}

impl<'a> EDUContentRef<'a> for UnknownContent {
    fn upcast(self) -> AnyEDUContentRef<'a> {
        AnyEDUContentRef::Unknown(self)
    }
}

impl<'a> EDUContentRef<'a> for AnyEDUContentRef<'a> {
    fn upcast(self) -> AnyEDUContentRef<'a> {
        self
    }
}

impl<'a> Display for EDURef<'a, AnyEDUContentRef<'a>> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: ", self.edu_type)?;

        match &self.content {
            AnyEDUContentRef::Typing(typing) => {
                write!(f, "{} in {}", typing.user_id, typing.room_id)
            }
            AnyEDUContentRef::Presence(presence) => {
                let mut first = true;
                for push_event in &presence.push {
                    if first {
                        first = false;
                    } else {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}={}", push_event.user_id, push_event.presence)?;
                }
                Ok(())
            },
            AnyEDUContentRef::Receipt(receipt) => {
                write!(f, "{} room receipts", receipt.room_receipts.len())
            },
            AnyEDUContentRef::Unknown(_) => {
                write!(f, "unknown EDU")
            }
        }
    }
}