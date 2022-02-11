use std::borrow::Cow;

use fluctlight_mod_interface::{ProcessRequestFunc, Request};
use libloading::{Library, Symbol};
use tokio::sync::RwLock;

use crate::error::Result;

pub(crate) struct MainModule {
    library: RwLock<Option<Library>>,
}

impl MainModule {
    pub(crate) fn new() -> Result<Self> {
        let library = unsafe { Library::new("target/debug/libfluctlight_router.so")? };
        Ok(MainModule {
            library: RwLock::new(Some(library)),
        })
    }

    pub(crate) async fn process_request(
        &self,
        uri: &str,
        method: &str,
    ) -> Result<(u16, Cow<'static, str>)> {
        if uri == "/restart" {
            self.restart().await?;

            Ok((200, "Restarted.\n".into()))
        } else {
            let library = self.library.read().await;

            // SAFETY: The library is trusted, and uses abi_stable
            // Although if `ProcessRequestFunc`'s types are out of sync, all
            // hell will break loose.
            let response = unsafe {
                let entry_point: Symbol<ProcessRequestFunc> = library
                    .as_ref()
                    .ok_or("Module not loaded")?
                    .get(b"process_request")
                    .map_err(|err| {
                        format!(
                            "Could not load process_request symbol from library: {}",
                            err
                        )
                    })?;
                entry_point(Request::new(uri, method))
            };

            let response = response.into_result()?;

            Ok(response.into())
        }
    }

    pub(crate) async fn restart(&self) -> Result<()> {
        eprintln!("Acquiring module lock...");

        let mut library = self.library.write().await;

        eprintln!("Restarting...");

        library
            .take()
            .ok_or("Module not loaded")?
            .close()
            .map_err(|err| format!("Could not close module: {}", err))?;
        *library = Some(unsafe {
            Library::new("target/debug/libfluctlight_router.so")
                .map_err(|err| format!("Could not load module: {}", err))?
        });

        eprintln!("Done.");

        Ok(())
    }
}
