use std::{any::Any, borrow::Cow};

use abi_stable::{
    erased_types::TypeInfo,
    std_types::{RBox, RCow, RSlice, RStr, RString},
    DynTrait, ImplType, StableAbi,
};

pub use abi_stable::std_types::RResult;

#[derive(StableAbi)]
#[repr(C)]
pub struct Request<'a> {
    module_state: &'a OpaqueModuleState,
    uri: RStr<'a>,
    method: RStr<'a>,
    body: RSlice<'a, u8>,
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
    content_type: RStr<'static>,
    body: RCow<'static, [u8]>,
}

pub struct ModuleState {
    pub state: Box<dyn Any + Send + Sync>,
}

#[derive(StableAbi)]
#[repr(C)]
pub struct OpaqueModuleState {
    opaque_state: DynTrait<'static, RBox<()>, ModuleStateInterface>,
}

impl ModuleState {
    pub fn into_opaque(self) -> OpaqueModuleState {
        OpaqueModuleState {
            opaque_state: DynTrait::from_value(self),
        }
    }

    pub fn from_opaque(opaque: OpaqueModuleState) -> Self {
        RBox::into_inner(
            opaque
                .opaque_state
                .downcast_into()
                .expect("Could not downcast module state"),
        )
    }

    pub fn as_inner(opaque: &OpaqueModuleState) -> &Self {
        opaque
            .opaque_state
            .downcast_as()
            .expect("Could not downcast module state")
    }
}

impl ImplType for ModuleState {
    type Interface = ModuleStateInterface;

    const INFO: &'static TypeInfo = abi_stable::impl_get_type_info!(ModuleState);
}

#[repr(C)]
#[derive(StableAbi)]
#[sabi(impl_InterfaceType(Send, Sync))]
pub struct ModuleStateInterface;

pub type ProcessRequestFunc<'a> = unsafe extern "C" fn(Request<'a>) -> ResponseResult;
pub type CreateStateFunc<'a> = unsafe extern "C" fn() -> OpaqueModuleState;
pub type DestroyStateFunc<'a> = unsafe extern "C" fn(OpaqueModuleState) -> bool;

impl<'a> Request<'a> {
    pub fn new(
        module_state: &'a OpaqueModuleState,
        uri: &'a str,
        method: &'a str,
        body: &'a [u8],
    ) -> Self {
        Request {
            module_state,
            uri: uri.into(),
            method: method.into(),
            body: body.into(),
        }
    }

    pub fn module_state(&self) -> &'a ModuleState {
        ModuleState::as_inner(self.module_state)
    }

    pub fn uri(&self) -> &'a str {
        self.uri.into()
    }

    pub fn method(&self) -> &'a str {
        self.method.into()
    }

    pub fn body(&self) -> &'a [u8] {
        self.body.into()
    }
}

impl From<(u16, &'static str, Cow<'static, [u8]>)> for Response {
    fn from((status, content_type, body): (u16, &'static str, Cow<'static, [u8]>)) -> Self {
        Response {
            status,
            content_type: content_type.into(),
            body: body.into(),
        }
    }
}

impl From<Response> for (u16, &'static str, Cow<'static, [u8]>) {
    fn from(response: Response) -> Self {
        (
            response.status,
            response.content_type.into(),
            response.body.into(),
        )
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
    pub fn new(status: u16, content_type: &'static str, body: Cow<'static, [u8]>) -> Self {
        (status, content_type, body).into()
    }
}

impl ResponseResult {
    pub fn into_result(self) -> Result<Response, String> {
        self.response.into_result().map_err(Into::into)
    }
}
