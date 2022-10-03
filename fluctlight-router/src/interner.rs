use std::{borrow::Borrow, collections::BTreeSet, fmt::Display, ops::Deref, sync::Arc};

use crate::matrix_types::{Event, Id, Key, Room, ServerName, User};

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

impl<T: ?Sized> TypedInterner<T> {
    pub(crate) fn new() -> Self {
        TypedInterner {
            interned_strings: BTreeSet::new(),
            next_id: 0,
        }
    }
}

/*
impl<I> TypedInterner<Id<I>> {
    pub(crate) fn get_or_insert(&mut self, value: &Id<I>) -> ArcStr<Id<I>> {
        if let Some(interned_value) = self.interned_strings.get(value) {
            interned_value.clone()
        } else {
            let new_value = ArcStr {
                int_str: IntStr {
                    id: self.next_id,
                    phantom: Default::default(),
                },
                inner: value.to_arc(),
            };
            self.interned_strings.insert(new_value.clone());
            new_value
        }
    }
}
*/

impl<T: Internable + ?Sized> TypedInterner<T> {
    pub(crate) fn get_or_insert(&mut self, value: &T) -> ArcStr<T> {
        if let Some(interned_value) = self.interned_strings.get(value) {
            interned_value.clone()
        } else {
            let new_value = ArcStr {
                int_str: IntStr {
                    id: self.next_id,
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
}

impl Interner {
    pub(crate) fn new() -> Self {
        Interner {
            event_interner: TypedInterner::new(),
            user_interner: TypedInterner::new(),
            server_name_interner: TypedInterner::new(),
            room_interner: TypedInterner::new(),
            key_interner: TypedInterner::new(),
            str_interner: TypedInterner::new(),
        }
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
