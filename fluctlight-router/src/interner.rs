use std::{borrow::Borrow, collections::BTreeSet, fmt::Display, sync::Arc};

use crate::matrix_types::Id;

pub(crate) struct ArcStr<T: ?Sized> {
    int_str: IntStr<T>,
    inner: Arc<T>,
}

#[derive(PartialEq, Eq)]
pub(crate) struct IntStr<T: ?Sized> {
    id: usize,
    phantom: std::marker::PhantomData<T>,
}

pub(crate) struct Interner<T: ?Sized> {
    interned_strings: BTreeSet<ArcStr<T>>,
    next_id: usize,
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

impl<T: ?Sized> Interner<T> {
    pub(crate) fn new() -> Self {
        Interner {
            interned_strings: BTreeSet::new(),
            next_id: 0,
        }
    }
}

impl<I> Interner<Id<I>> {
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
