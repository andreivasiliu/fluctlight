use std::borrow::Cow;

use fluctlight_mod_interface::{ProcessRequestFunc, Request};
use libloading::{Library, Symbol};
use tokio::sync::RwLock;

use crate::error::Result;

pub(crate) struct MainModule {
    library: RwLock<Library>,
}

impl MainModule {
    pub(crate) fn new() -> Result<Self> {
        let library = unsafe { Library::new("target/debug/libfluctlight_router.so")? };
        Ok(MainModule {
            library: RwLock::new(library),
        })
    }

    pub(crate) async fn process_request(&self, uri: &str) -> (u16, Cow<'static, str>) {
        let library = self.library.read().await;

        // SAFETY: The library is trusted, and uses abi_stable
        let response = unsafe {
            let entry_point: Symbol<ProcessRequestFunc> = library
                .get(b"process_request")
                .expect("Library should contain process_request symbol");
            entry_point(Request::new(uri))
        };

        response.into()
    }
}
