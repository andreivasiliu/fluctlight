use std::borrow::Cow;

use abi_stable::{
    std_types::{RCow, RStr, RString},
    StableAbi,
};

pub use abi_stable::std_types::RResult;

#[derive(StableAbi)]
#[repr(C)]
pub struct Request<'a> {
    uri: RStr<'a>,
    method: RStr<'a>,
}

#[derive(StableAbi)]
#[repr(C)]
pub struct ResponseResult {
    response: RResult<Response, RString>,
}

#[derive(StableAbi)]
#[repr(C)]
pub struct Response {
    status: u16,
    body: RCow<'static, str>,
}

pub type ProcessRequestFunc<'a> = unsafe extern "C" fn(Request<'a>) -> ResponseResult;

impl<'a> Request<'a> {
    pub fn new(uri: &'a str, method: &'a str) -> Self {
        Request {
            uri: uri.into(),
            method: method.into(),
        }
    }

    pub fn uri(&self) -> &'a str {
        self.uri.into()
    }

    pub fn method(&self) -> &'a str {
        self.method.into()
    }
}

impl From<(u16, Cow<'static, str>)> for Response {
    fn from((status, body): (u16, Cow<'static, str>)) -> Self {
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

impl From<Result<Response, String>> for ResponseResult {
    fn from(result: Result<Response, String>) -> Self {
        ResponseResult {
            response: result
                .map(|response| response.into())
                .map_err(|err| err.into())
                .into(),
        }
    }
}

impl Response {
    pub fn new(status: u16, body: Cow<'static, str>) -> Self {
        (status, body).into()
    }
}

impl ResponseResult {
    pub fn into_result(self) -> Result<Response, String> {
        self.response.into_result().map_err(Into::into)
    }
}
