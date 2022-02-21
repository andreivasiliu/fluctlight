use bumpalo::{collections::CollectIn, Bump};
use fluctlight_mod_interface::{Request, Response};
use serde::{de::MapAccess, forward_to_deserialize_any, Deserialize, Deserializer, Serialize};
use smallvec::SmallVec;

use crate::{routes_federation::federation_api_handler, state::State};

pub(crate) struct RequestData<'a> {
    pub memory_pool: &'a Bump,
    pub state: &'a State,
    pub http_request: http::Request<&'a [u8]>,
}

type BumpString<'a> = bumpalo::collections::String<'a>;

impl<'r> RequestData<'r> {
    pub fn new_str(&self, s: &str) -> &'r str {
        BumpString::from_str_in(s, self.memory_pool).into_bump_str()
    }

    pub fn handle_with<F, Path, QueryString, Body>(
        &'r self,
        handler: F,
    ) -> Result<http::Response<Vec<u8>>, String>
    where
        F: Fn(
            &RequestData<'r>,
            GenericRequest<Path, QueryString, Body>,
        ) -> <GenericRequest<Path, QueryString, Body> as MatrixRequest>::Response,
        GenericRequest<Path, QueryString, Body>: MatrixRequest,
        Path: Deserialize<'r>,
        QueryString: Deserialize<'r>,
        Body: Deserialize<'r>,
        <GenericRequest<Path, QueryString, Body> as MatrixRequest>::Response: Serialize,
    {
        let body = if self.http_request.method() == "GET" {
            b"{}".as_slice()
        } else {
            self.http_request.body()
        };

        type BumpVec<'b, T> = bumpalo::collections::Vec<'b, T>;

        let path = self.http_request.uri().path();
        let path_segments: BumpVec<_> = path.split('/').collect_in(self.memory_pool);
        let spec = <GenericRequest<Path, QueryString, Body> as MatrixRequest>::PATH_SPEC;
        let spec_segments: BumpVec<_> = spec.split('/').collect_in(self.memory_pool);

        let mut path_deserializer = RequestPathDeserializer {
            path_segments: &path_segments,
            spec_segments: &spec_segments,
            next_value: None,
        };

        let request_path = Path::deserialize(&mut path_deserializer)
            .map_err(|err| format!("Could not deserialize the request path: {}", err))?;
        let request_qs = serde_json::from_slice(b"{}")
            .map_err(|err| format!("Could not deserialize the request's query string: {}", err))?;
        let request_body = serde_json::from_slice(body)
            .map_err(|err| format!("Could not deserialize the request body: {}", err))?;

        let request = GenericRequest::new(request_path, request_qs, request_body);
        let response = handler(self, request);
        let mut response_bytes = serde_json::to_vec(&response)
            .map_err(|err| format!("Could not serialize response: {}", err))?;
        response_bytes.push(b'\n');
        let http_response = http::Response::builder()
            .body(response_bytes)
            .expect("Response should always be valid");
        Ok(http_response)
    }
}

pub(super) fn try_process_request<'a>(
    state: &State,
    request: Request<'a>,
) -> Result<Response, String> {
    let mut uri_segments: SmallVec<[&str; 8]> = request.uri().split('/').collect();
    uri_segments[0] = request.method();

    let http_request = http::Request::builder()
        .method(request.method())
        .uri(request.uri())
        .body(request.body())
        .expect("Request should always be valid");

    let memory_pool = bumpalo::Bump::with_capacity(256);
    let request_data = RequestData {
        memory_pool: &memory_pool,
        state: &state,
        http_request,
    };

    let http_response = if let Some(http_response) =
        federation_api_handler(uri_segments.as_slice(), &request_data)
    {
        http_response
    } else if let Some(http_response) =
        federation_api_handler(uri_segments.as_slice(), &request_data)
    {
        http_response
    } else {
        return Ok(Response::new(404, b"Not found\n".as_slice().into()));
    };

    let http_response =
        http_response.map_err(|err| format!("Could not process request: {}", err))?;
    return Ok(Response::new(200, http_response.into_body().into()));
}

pub(crate) struct GenericRequest<Path, QueryString, Body> {
    pub path: Path,
    pub query_string: QueryString,
    pub body: Body,
}

impl<'r, Path, QueryString, Body> GenericRequest<Path, QueryString, Body> {
    pub fn new(path: Path, query_string: QueryString, body: Body) -> Self {
        GenericRequest {
            path,
            query_string,
            body,
        }
    }
}

pub(crate) trait MatrixRequest {
    type Response;
    const PATH_SPEC: &'static str;
}

#[derive(Serialize, Deserialize)]
pub(crate) struct EmptyPath<'a> {
    #[serde(skip)]
    phantom: std::marker::PhantomData<&'a ()>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct EmptyBody {}

#[derive(Serialize, Deserialize)]
pub(crate) struct EmptyQS {}

#[derive(Debug)]
struct RequestDeserializationError(String);

impl std::fmt::Display for RequestDeserializationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

impl std::error::Error for RequestDeserializationError {}

impl serde::de::Error for RequestDeserializationError {
    fn custom<T>(msg: T) -> Self
    where
        T: std::fmt::Display,
    {
        RequestDeserializationError(msg.to_string())
    }
}

struct RequestPathDeserializer<'de, 'a> {
    path_segments: &'a [&'de str],
    spec_segments: &'a [&'static str],
    next_value: Option<Option<&'de str>>,
}

impl<'de, 'a> Deserializer<'de> for &'a mut RequestPathDeserializer<'de, 'a> {
    type Error = RequestDeserializationError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        visitor.visit_map(RequestPathMapAccess(self))
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

struct RequestPathMapAccess<'de, 'a>(&'a mut RequestPathDeserializer<'de, 'a>);

impl<'de, 'a> MapAccess<'de> for RequestPathMapAccess<'de, 'a> {
    type Error = RequestDeserializationError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: serde::de::DeserializeSeed<'de>,
    {
        loop {
            let spec_segment = self.0.spec_segments.first();
            let path_segment = self.0.path_segments.first();

            match (spec_segment, path_segment) {
                (Some(spec_segment), Some(path_segment)) => {
                    self.0.spec_segments = &self.0.spec_segments[1..];
                    self.0.path_segments = &self.0.path_segments[1..];

                    let is_variable = spec_segment.starts_with(":");
                    let is_optional_variable = spec_segment.starts_with("?");

                    if is_variable || is_optional_variable {
                        let variable_name = Some(&spec_segment[1..]);
                        self.0.next_value = Some(Some(path_segment));
                        return seed
                            .deserialize(&mut RequestPathFieldDeserializer(variable_name))
                            .map(Some);
                    } else if spec_segment == path_segment {
                        continue;
                    } else {
                        return Err(RequestDeserializationError(String::from(
                            "Path does not match the requested path specification",
                        )));
                    }
                }
                (None, Some(path_segment)) => {
                    return Err(RequestDeserializationError(format!(
                        "Unexpected segment in URI: '{}'",
                        path_segment
                    )));
                }
                (Some(spec_segment), None) => {
                    if spec_segment.starts_with("?") {
                        self.0.spec_segments = &self.0.spec_segments[1..];

                        let variable_name = Some(&spec_segment[1..]);
                        // Deserialize `None` values for missing segments
                        self.0.next_value = Some(None);
                        return seed
                            .deserialize(&mut RequestPathFieldDeserializer(variable_name))
                            .map(Some);
                    } else {
                        return Err(RequestDeserializationError(format!(
                            "Missing segment in URI path for {}",
                            spec_segment
                        )));
                    }
                }
                (None, None) => return Ok(None),
            }
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::DeserializeSeed<'de>,
    {
        let value = self
            .0
            .next_value
            .take()
            .expect("Guaranteed by next_key_seed()");
        seed.deserialize(&mut RequestPathFieldDeserializer(value))
    }
}

struct RequestPathFieldDeserializer<'de>(Option<&'de str>);

impl<'de, 'a> Deserializer<'de> for &'a mut RequestPathFieldDeserializer<'de> {
    type Error = RequestDeserializationError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.0 {
            Some(value) => visitor.visit_borrowed_str(value),
            None => visitor.visit_none(),
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: serde::de::Visitor<'de>,
    {
        match self.0 {
            Some(_value) => visitor.visit_some(self),
            None => visitor.visit_none(),
        }
    }

    forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}
