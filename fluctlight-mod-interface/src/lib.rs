use std::borrow::Cow;

use abi_stable::{StableAbi, std_types::{RStr, RString, RCow}};

#[derive(StableAbi)]
#[repr(C)]
pub struct Request<'a> {
    uri: RStr<'a>,
}

#[derive(StableAbi)]
#[repr(C)]
pub struct Response {
    status: u16,
    body: RCow<'static, str>,
}

pub type ProcessRequestFunc<'a> = unsafe extern "C" fn(Request<'a>) -> Response;


impl<'a> Request<'a> {
    pub fn new(uri: &'a str) -> Self {
        Request {
            uri: uri.into()
        }
    }

    pub fn uri(&self) -> &'a str {
        self.uri.into()
    }
}

impl Response {
    pub fn new(status: u16, body: Cow<'static, str>) -> Self {
        Response {
            status,
            body: body.into(),
        }
    }
}

impl From<Response> for (u16, Cow<'static, str>) {
    fn from(response: Response) -> Self {
        (response.status, response.body.into())
    }
}