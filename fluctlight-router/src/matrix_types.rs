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
    fn as_str(&self) -> &str {
        &self.content
    }
}

impl<T: MatrixId> Id<T> {
    pub fn try_from_str(string: &str) -> Result<&Self, String> {
        T::validate(string)?;
        // SAFETY: repr(transparent) and the phantom's zero-size make these
        // types equivalent.
        Ok(unsafe { std::mem::transmute(string) })
    }

    pub fn try_boxed_from_str(string: &str) -> Result<Box<Self>, String> {
        T::validate(string)?;

        let boxed: Box<str> = string.into();

        // SAFETY: repr(transparent) and the phantom's zero-size make these
        // types equivalent.
        Ok(unsafe { std::mem::transmute(boxed) })
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
