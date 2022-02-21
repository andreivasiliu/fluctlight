use std::borrow::Cow;

use serde::Serialize;

/// A raw JSON string that remembers the type it should deserialize into.
#[derive(Serialize)]
#[serde(transparent)]
pub(crate) struct RenderedJson<'a, T> {
    #[serde(skip)]
    phantom: std::marker::PhantomData<T>,
    bytes: Cow<'a, serde_json::value::RawValue>,
}

impl<T> RenderedJson<'static, T> {
    pub(crate) fn from_trusted(json_string: String) -> Self {
        let raw_value =
            serde_json::value::RawValue::from_string(json_string).expect("Trusted JSON source");
        RenderedJson {
            phantom: Default::default(),
            bytes: Cow::Owned(raw_value),
        }
    }
}
