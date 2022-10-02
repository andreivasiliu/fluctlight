use std::{
    fmt::{Debug, Display},
    sync::Arc,
};

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(transparent)]
#[repr(transparent)]
pub(crate) struct Id<T> {
    #[serde(skip)]
    phantom: std::marker::PhantomData<T>,
    content: str,
}

pub(crate) struct User;
pub(crate) struct Device;
pub(crate) struct Room;
pub(crate) struct Event;
pub(crate) struct ServerName;
pub(crate) struct Key;

pub(crate) trait MatrixId {
    fn validate(_string: &str) -> Result<(), String> {
        Ok(())
    }
}

impl MatrixId for User {}
impl MatrixId for Device {}
impl MatrixId for Room {}
impl MatrixId for Event {}
impl MatrixId for ServerName {}
impl MatrixId for Key {}

impl<T> Id<T> {
    pub fn as_str(&self) -> &str {
        &self.content
    }

    pub fn to_arc(&self) -> Arc<Self> {
        let str_value = self.as_str();
        let arc_value: Arc<str> = str_value.into();

        // SAFETY: repr(transparent) and the phantom's zero-size make these
        // types equivalent.
        unsafe { std::mem::transmute::<Arc<str>, Arc<Id<T>>>(arc_value) }
    }
}

impl<T: MatrixId> Id<T> {
    pub fn try_from_str(string: &str) -> Result<&Self, String> {
        T::validate(string)?;
        // SAFETY: repr(transparent) and the phantom's zero-size make these
        // types equivalent.
        Ok(unsafe { std::mem::transmute::<&str, &Id<T>>(string) })
    }

    pub fn try_boxed_from_str(string: &str) -> Result<Box<Self>, String> {
        T::validate(string)?;

        let boxed: Box<str> = string.into();

        // SAFETY: repr(transparent) and the phantom's zero-size make these
        // types equivalent.
        Ok(unsafe { std::mem::transmute::<Box<str>, Box<Id<T>>>(boxed) })
    }
}

impl<T> AsRef<str> for Id<T> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<T> PartialOrd for Id<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.as_str().partial_cmp(other.as_str())
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.as_str().eq(other.as_str())
    }
}

impl<T> Eq for Id<T> {}
impl<T> Ord for Id<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.as_str().cmp(other.as_str())
    }
}

impl<T: MatrixId> Clone for Box<Id<T>> {
    fn clone(&self) -> Self {
        Id::<T>::try_boxed_from_str(self.as_str()).expect("Already validated")
    }
}

impl<T: MatrixId> ToOwned for Id<T> {
    type Owned = Box<Id<T>>;

    fn to_owned(&self) -> Self::Owned {
        Id::<T>::try_boxed_from_str(self.as_str()).expect("Already validated")
    }
}

impl<T> Display for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.as_str(), f)
    }
}

impl<T> Debug for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.as_str(), f)
    }
}

impl<'de: 'a, 'a, T: MatrixId> Deserialize<'de> for &'a Id<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let string = <&str>::deserialize(deserializer)?;

        Id::try_from_str(string).map_err(|err| serde::de::Error::custom(err))
    }
}

impl<'de, T: MatrixId> Deserialize<'de> for Box<Id<T>> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let string = String::deserialize(deserializer)?;

        // FIXME: convert in place
        Id::try_boxed_from_str(&string).map_err(|err| serde::de::Error::custom(err))
    }
}

impl Id<User> {
    pub fn server_name(&self) -> &Id<ServerName> {
        let parts = self
            .as_str()
            .split_once(':')
            .expect("The name should already be validated");

        Id::<ServerName>::try_from_str(parts.1)
            .expect("The server part should already be validated")
    }
}
