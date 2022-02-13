use std::collections::BTreeMap;

use crate::matrix_types::{Id, User, Device};

pub(crate) struct State {
    pub(crate) users: BTreeMap<Box<Id<User>>, UserState>,
}

pub(crate) struct UserState {
    // pub(crate) name: Box<Id<User>>,
    // pub(crate) devices: Vec<Box<Id<Device>>>,
    pub(crate) keys: Vec<(Box<Id<Device>>, ())>,
}

impl State {
    pub(crate) fn new() -> Self {
        State { users: BTreeMap::new() }
    }
}
