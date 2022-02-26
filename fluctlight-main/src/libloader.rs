use std::{borrow::Cow, path::PathBuf, sync::Arc};

use fluctlight_mod_interface::{
    CreateStateFunc, DestroyStateFunc, OpaqueModuleState, ProcessRequestFunc, Request,
};
use hyper::{Method, Uri};
use libloading::{library_filename, Library, Symbol};
use tokio::{sync::RwLock, task::spawn_blocking};

use crate::error::Result;

pub(crate) struct MainModule {
    library: Arc<RwLock<LibraryAndState>>,
}

struct LibraryAndState(Option<(Library, OpaqueModuleState)>);

impl MainModule {
    pub(crate) fn library_name() -> PathBuf {
        let mut path = PathBuf::from("target");
        path.push(if cfg!(debug_assertions) {
            "debug"
        } else {
            "release"
        });
        path.push(library_filename("fluctlight_router"));
        path
    }

    pub(crate) fn new() -> Result<Self> {
        let library = unsafe { Library::new(Self::library_name())? };

        let create_state: Symbol<CreateStateFunc> = unsafe {
            library.get(b"create_state").map_err(|err| {
                format!("Could not load create_state symbol from library: {}", err)
            })?
        };

        let module_state = unsafe { create_state() };

        Ok(MainModule {
            library: Arc::new(RwLock::new(LibraryAndState(Some((library, module_state))))),
        })
    }

    pub(crate) async fn process_request(
        &self,
        uri: Uri,
        method: Method,
        body: Vec<u8>,
    ) -> (u16, Cow<'static, [u8]>) {
        if uri == "/restart" {
            eprintln!("Acquiring module write lock...");

            let mut module = self.library.write().await;
            let result = module
                .restart()
                .map(|()| (200, "Restarted.\n".as_bytes().into()));
            result_to_http_response(result)
        } else {
            let library = self.library.clone().read_owned().await;

            let response = spawn_blocking(move || {
                result_to_http_response(library.process_request(uri, method, body))
            });

            response.await.expect("Process handler should never panic")
        }
    }

    pub(crate) async fn restart(&self) -> Result<()> {
        eprintln!("Acquiring module lock...");

        let mut module = self.library.write().await;
        module.restart()
    }
}

fn result_to_http_response(result: Result<(u16, Cow<'static, [u8]>)>) -> (u16, Cow<'static, [u8]>) {
    match result {
        Ok(response) => response,
        Err(err) => {
            eprintln!("Fatal error: {}", err);
            (
                500,
                format!("Internal server error\n\n{}\n", err)
                    .into_bytes()
                    .into(),
            )
        }
    }
}

impl LibraryAndState {
    fn process_request(
        &self,
        uri: Uri,
        method: Method,
        body: Vec<u8>,
    ) -> Result<(u16, Cow<'static, [u8]>)> {
        let (library, module_state) = self.0.as_ref().ok_or("Module not loaded")?;

        // SAFETY: The library is trusted, and uses abi_stable
        // Although if `ProcessRequestFunc`'s types are out of sync, all
        // hell will break loose.
        let module_response = unsafe {
            let entry_point: Symbol<ProcessRequestFunc> =
                library.get(b"process_request").map_err(|err| {
                    format!(
                        "Could not load process_request symbol from library: {}",
                        err
                    )
                })?;
            entry_point(Request::new(
                module_state,
                uri.path(),
                method.as_str(),
                body.as_slice(),
            ))
        };
        Ok(module_response.into_result()?.into())
    }

    fn restart(&mut self) -> Result<()> {
        let module = self;

        eprintln!("Restarting...");

        let (library, module_state) = module.0.take().ok_or("Module not loaded")?;

        let destroy_state: Symbol<DestroyStateFunc> = unsafe {
            library.get(b"destroy_state").map_err(|err| {
                format!("Could not load destroy_state symbol from library: {}", err)
            })?
        };

        let destroyed = unsafe { destroy_state(module_state) };

        if !destroyed {
            return Err("Could not destroy old module's state; restart aborted.".into());
        }

        library
            .close()
            .map_err(|err| format!("Could not close module: {}", err))?;
        let library = unsafe {
            Library::new(MainModule::library_name())
                .map_err(|err| format!("Could not load module: {}", err))?
        };

        let create_state: Symbol<CreateStateFunc> = unsafe {
            library.get(b"create_state").map_err(|err| {
                format!("Could not load create_state symbol from library: {}", err)
            })?
        };

        let module_state = unsafe { create_state() };

        *module = LibraryAndState(Some((library, module_state)));

        eprintln!("Done.");

        Ok(())
    }
}

// Destroy the state before unloading the library.
impl Drop for LibraryAndState {
    fn drop(&mut self) {
        let (library, state) = match self.0.take() {
            Some(module) => module,
            None => return,
        };

        // Ask the module to drop it so that panics don't cross the FFI boundary
        let destroy_state: Symbol<DestroyStateFunc> = unsafe {
            library
                .get(b"destroy_state")
                .expect("Could not load destroy_state symbol from library")
        };

        unsafe { destroy_state(state) };

        drop(library);
    }
}
