use std::{borrow::Borrow, collections::BTreeSet, fmt::Display, ops::Deref, sync::Arc};

use crate::matrix_types::{Event, Id, Key, Room, ServerName, User};

#[derive(Default)]
pub(crate) struct Interner {
    event_interner: TypedInterner<Id<Event>>,
    user_interner: TypedInterner<Id<User>>,
    server_name_interner: TypedInterner<Id<ServerName>>,
    room_interner: TypedInterner<Id<Room>>,
    key_interner: TypedInterner<Id<Key>>,
    str_interner: TypedInterner<str>,
}

pub(crate) struct TypedInterner<T: ?Sized> {
    interned_strings: BTreeSet<ArcStr<T>>,
    next_id: usize,
}

pub(crate) struct ArcStr<T: ?Sized> {
    int_str: IntStr<T>,
    inner: Arc<T>,
}

#[derive(PartialEq, Eq)]
pub(crate) struct IntStr<T: ?Sized> {
    id: usize,
    phantom: std::marker::PhantomData<T>,
}

impl<T: ?Sized> Default for TypedInterner<T> {
    fn default() -> Self {
        Self {
            interned_strings: Default::default(),
            next_id: Default::default(),
        }
    }
}

impl<T: ?Sized + PartialEq> PartialEq for ArcStr<T> {
    fn eq(&self, other: &Self) -> bool {
        self.int_str == other.int_str
    }
}

impl<T: ?Sized + PartialOrd> PartialOrd for ArcStr<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.inner.partial_cmp(&other.inner)
    }
}

impl<T: ?Sized + Eq> Eq for ArcStr<T> {}
impl<T: ?Sized + Ord> Ord for ArcStr<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.inner.cmp(&other.inner)
    }
}

impl<T: ?Sized> Clone for ArcStr<T> {
    fn clone(&self) -> Self {
        Self {
            int_str: self.int_str.clone(),
            inner: self.inner.clone(),
        }
    }
}

impl<T: ?Sized> Clone for IntStr<T> {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            phantom: Default::default(),
        }
    }
}

impl<T: ?Sized> Borrow<T> for ArcStr<T> {
    fn borrow(&self) -> &T {
        &self.inner
    }
}

impl<T: ?Sized + Display> Display for ArcStr<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.inner.fmt(f)
    }
}

impl<T: ?Sized> Deref for ArcStr<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<T: Internable + ?Sized> TypedInterner<T> {
    pub(crate) fn get_or_insert(&mut self, value: &T) -> ArcStr<T> {
        if let Some(interned_value) = self.interned_strings.get(value) {
            interned_value.clone()
        } else {
            if self.next_id == usize::MAX {
                panic!("Reached interner's limit!");
            }
            let id = self.next_id;
            self.next_id += 1;
            let new_value = ArcStr {
                int_str: IntStr {
                    id,
                    phantom: Default::default(),
                },
                inner: value.to_arc(),
            };
            self.interned_strings.insert(new_value.clone());
            new_value
        }
    }
}

pub(crate) trait Internable: Ord + ToArc {
    fn get_typed_interner(interner: &mut Interner) -> &mut TypedInterner<Self>;
}

impl Interner {
    pub(crate) fn get_or_insert<T: Internable + ?Sized>(&mut self, value: &T) -> ArcStr<T> {
        let typed_interner = T::get_typed_interner(self);
        typed_interner.get_or_insert(value)
    }

    pub(crate) fn print_memory_usage(&self) {
        let lengths = [
            ("events", self.event_interner.interned_strings.len()),
            ("users", self.user_interner.interned_strings.len()),
            ("servers", self.server_name_interner.interned_strings.len()),
            ("rooms", self.room_interner.interned_strings.len()),
            ("keys", self.key_interner.interned_strings.len()),
            ("strings", self.str_interner.interned_strings.len()),
        ];
        let mut total = 0;

        println!("Interned strings:");

        for (name, length) in lengths {
            println!("* {}: {}", name, length);
            total += length;
        }

        let mut total_bytes: usize = 0;

        total_bytes += self
            .event_interner
            .interned_strings
            .iter()
            .map(|s| s.as_str().len())
            .sum::<usize>();
        total_bytes += self
            .user_interner
            .interned_strings
            .iter()
            .map(|s| s.as_str().len())
            .sum::<usize>();
        total_bytes += self
            .server_name_interner
            .interned_strings
            .iter()
            .map(|s| s.as_str().len())
            .sum::<usize>();
        total_bytes += self
            .room_interner
            .interned_strings
            .iter()
            .map(|s| s.as_str().len())
            .sum::<usize>();
        total_bytes += self
            .key_interner
            .interned_strings
            .iter()
            .map(|s| s.as_str().len())
            .sum::<usize>();
        total_bytes += self
            .str_interner
            .interned_strings
            .iter()
            .map(|s| s.len())
            .sum::<usize>();

        println!(
            "Total: {} strings, with {} overhead",
            total,
            total * std::mem::size_of::<Arc<ArcStr<str>>>(),
        );

        println!("Total bytes by interned strings: {}", total_bytes);
    }
}

impl Interner {
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

pub(crate) trait ToArc {
    fn to_arc(&self) -> Arc<Self>;
}

impl<T> ToArc for Id<T> {
    fn to_arc(&self) -> Arc<Self> {
        self.to_arc()
    }
}

impl ToArc for str {
    fn to_arc(&self) -> Arc<Self> {
        self.into()
    }
}

impl Internable for Id<Event> {
    fn get_typed_interner(interner: &mut Interner) -> &mut TypedInterner<Self> {
        &mut interner.event_interner
    }
}

impl Internable for Id<User> {
    fn get_typed_interner(interner: &mut Interner) -> &mut TypedInterner<Self> {
        &mut interner.user_interner
    }
}

impl Internable for Id<ServerName> {
    fn get_typed_interner(interner: &mut Interner) -> &mut TypedInterner<Self> {
        &mut interner.server_name_interner
    }
}

impl Internable for Id<Room> {
    fn get_typed_interner(interner: &mut Interner) -> &mut TypedInterner<Self> {
        &mut interner.room_interner
    }
}

impl Internable for Id<Key> {
    fn get_typed_interner(interner: &mut Interner) -> &mut TypedInterner<Self> {
        &mut interner.key_interner
    }
}

impl Internable for str {
    fn get_typed_interner(interner: &mut Interner) -> &mut TypedInterner<Self> {
        &mut interner.str_interner
    }
}
