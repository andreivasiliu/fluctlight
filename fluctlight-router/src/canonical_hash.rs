use std::borrow::Cow;

use serde::{
    de::Visitor,
    ser::{SerializeMap, SerializeSeq},
    Deserialize, Serialize,
};
use sha2::Digest;
use smallvec::SmallVec;

pub(crate) fn verify_content_hash(json_blob: &str, print_canonical: bool) -> Result<(), String> {
    let mut value: ValueRef = serde_json::from_str(json_blob)
        .map_err(|err| format!("Could not deserialize error: {}", err))?;

    let existing_hashes = value
        .pop_from_map("hashes")
        .ok_or_else(|| "Expected PDU to have 'hashes' key".to_string())?;

    let existing_hash = existing_hashes
        .get_from_map("sha256")
        .ok_or_else(|| "Expected PDU hashes to have 'sha256' key".to_string())?;

    let existing_hash = match existing_hash {
        ValueRef::String(v) => v,
        _ => return Err("Expected sha256 value to be a string".to_string()),
    };

    value.pop_from_map("age_ts");
    value.pop_from_map("signatures");
    value.pop_from_map("event_id");
    let unsigned = value.pop_from_map("unsigned");

    let redacted = unsigned
        .map(|unsigned| unsigned.get_from_map("redacted_by").is_some())
        .unwrap_or(false);

    if redacted {
        return Ok(());
    }

    if print_canonical {
        let blob = serde_json::to_string(&value).expect("Serialization should always succeed");
        eprintln!("Computed canonical hash: {}", blob);

        return Ok(());
    }

    let mut hasher = sha2::Sha256::new();
    serde_json::to_writer(&mut hasher, &value).expect("Serialization should always succeed");
    let sha256_hash = hasher.finalize();

    let mut scratch_buffer = SmallVec::<[u8; 64]>::new();
    scratch_buffer.resize(64, 0);
    let hash_size = base64::encode_config_slice(
        sha256_hash.as_slice(),
        base64::STANDARD_NO_PAD,
        &mut scratch_buffer[..],
    );
    let computed_hash: &str =
        std::str::from_utf8(&scratch_buffer[..hash_size]).expect("Base64 is always a string");

    if existing_hash != computed_hash {
        return Err(format!(
            "PDU hash verification failed: {} (existing) vs {} (computed)",
            existing_hash, computed_hash
        ));
    }

    Ok(())
}

#[derive(PartialEq, Eq)]
enum ValueRef<'a> {
    Null,
    Boolean(bool),
    Number(i64),
    String(Cow<'a, str>),
    List(Vec<ValueRef<'a>>),
    Map(Vec<(ValueRef<'a>, ValueRef<'a>)>),
}

impl<'a> ValueRef<'a> {
    fn get_from_map(&self, key: &str) -> Option<&ValueRef<'a>> {
        let vec_map = match self {
            ValueRef::Map(map) => map,
            _ => return None,
        };

        for (map_key, value) in vec_map {
            match map_key {
                ValueRef::String(map_key) => {
                    if map_key == key {
                        return Some(value);
                    }
                }
                _ => continue,
            }
        }

        None
    }

    fn pop_from_map(&mut self, key: &str) -> Option<ValueRef<'a>> {
        let vec_map = match self {
            ValueRef::Map(map) => map,
            _ => return None,
        };

        for (index, (map_key, _value)) in vec_map.iter().enumerate() {
            match map_key {
                ValueRef::String(map_key) => {
                    if map_key == key {
                        let (_key, value) = vec_map.remove(index);
                        return Some(value);
                    }
                }
                _ => continue,
            }
        }

        None
    }
}

impl<'de> Deserialize<'de> for ValueRef<'de> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(ValueRefVisitor)
    }
}

struct ValueRefVisitor;

impl<'de> Visitor<'de> for ValueRefVisitor {
    type Value = ValueRef<'de>;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a JSON value (number, string, list, or map)")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if !(-2i64.pow(53) + 1..=2i64.pow(53) - 1).contains(&v) {
            Err(E::custom(format!(
                "Integers must be between -(2**53)+1 and (2**53)-1: got {} instead",
                v
            )))
        } else {
            Ok(ValueRef::Number(v))
        }
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        if v > 2u64.pow(53) - 1 {
            Err(E::custom(format!(
                "Integers must be between -(2**53)+1 and (2**53)-1: got {} instead",
                v
            )))
        } else {
            Ok(ValueRef::Number(v as i64))
        }
    }

    fn visit_f64<E>(self, _v: f64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Err(E::custom(
            "Floating point values are not allowed in Canonical JSON",
        ))
    }

    fn visit_borrowed_str<E>(self, v: &'de str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        // FIXME: bytes
        // FIXME: encodings
        Ok(ValueRef::String(Cow::Borrowed(v)))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        self.visit_string(v.to_string())
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ValueRef::String(Cow::Owned(v)))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut list = Vec::new();
        while let Some(value) = seq.next_element()? {
            list.push(value);
        }

        Ok(ValueRef::List(list))
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut vecmap = Vec::new();
        while let Some((key, value)) = map.next_entry()? {
            vecmap.push((key, value));
        }
        vecmap.sort_unstable_by(|(key1, _value1), (key2, _value2)| {
            let key1 = match key1 {
                ValueRef::String(ref v) => v,
                _ => "",
            };
            let key2 = match key2 {
                ValueRef::String(ref v) => v,
                _ => "",
            };
            key1.cmp(key2)
        });

        Ok(ValueRef::Map(vecmap))
    }

    fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ValueRef::Boolean(v))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(ValueRef::Null)
    }
}

impl<'de> Serialize for ValueRef<'de> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            ValueRef::Null => serializer.serialize_unit(),
            ValueRef::Boolean(v) => serializer.serialize_bool(*v),
            ValueRef::Number(v) => serializer.serialize_i64(*v),
            ValueRef::String(v) => serializer.serialize_str(v),
            ValueRef::List(list) => {
                let mut seq = serializer.serialize_seq(Some(list.len()))?;
                for value in list {
                    seq.serialize_element(value)?;
                }
                seq.end()
            }
            ValueRef::Map(vecmap) => {
                let mut map = serializer.serialize_map(Some(vecmap.len()))?;
                for (key, value) in vecmap {
                    map.serialize_entry(key, value)?;
                }
                map.end()
            }
        }
    }
}
