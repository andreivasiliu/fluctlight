use std::{convert::Infallible, net::SocketAddr, path::PathBuf, sync::Arc};

use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use libloading::library_filename;
use tokio_inotify::{AsyncINotify, IN_CREATE};

use crate::error::Result;
use crate::libloader::MainModule;

mod error;
mod libloader;

fn main() -> Result<()> {
    eprintln!("Starting runtime...");
    let tokio_runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap();

    eprintln!("Creating server...");
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    let main_module = Arc::new(MainModule::new().unwrap());

    tokio_runtime.spawn(watch_module(main_module.clone()));

    let make_server = async {
        eprintln!("Loading module...");

        let make_service = make_service_fn(move |_conn| {
            let main_module = main_module.clone();
            async {
                Ok::<_, Infallible>(service_fn(move |request: Request<Body>| {
                    let main_module = main_module.clone();
                    hello_world(main_module, request)
                }))
            }
        });

        std::result::Result::<_, hyper::Error>::Ok(
            Server::try_bind(&addr)?
                .serve(make_service)
                .with_graceful_shutdown(shutdown_signal()),
        )
    };

    let server = tokio_runtime.block_on(make_server)?;

    eprintln!("Server is ready.");
    tokio_runtime.block_on(server)?;

    eprintln!("Shutdown complete.");

    Ok(())
}

async fn watch_module(main_module: Arc<MainModule>) {
    let inotify = AsyncINotify::init().expect("Failed to install inotify watcher");

    let mut module_path = PathBuf::from("target");
    module_path.push(if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    });
    let module_name = library_filename("fluctlight_router");
    inotify
        .add_watch(&module_path, IN_CREATE)
        .expect("Failed to watch module path with inotify");

    eprintln!(
        "Installed watch on {} for {}",
        module_path.display(),
        module_name.to_string_lossy()
    );

    use futures_util::compat::Stream01CompatExt;
    use futures_util::StreamExt;
    let mut inotify_stream = inotify.compat();

    while let Some(event) = inotify_stream.next().await {
        let event = event.expect("Failed to get inotify event");

        if event.is_create() && event.name.ends_with(&module_name) {
            eprintln!("Received inotify event...");
            if let Err(err) = main_module.restart().await {
                eprintln!("Could not restart module on inotify even2t: {}", err);
            }
        }
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("Failed to install interrupt signal handler");

    eprintln!("\nInterrupt signal received, shutting down...")
}

async fn hello_world(
    main_module: Arc<MainModule>,
    req: Request<Body>,
) -> std::result::Result<Response<Body>, Infallible> {
    let (status, body) = match main_module
        .process_request(req.uri().path(), req.method().as_str())
        .await
    {
        Ok(response) => response,
        Err(err) => {
            eprintln!("Fatal error: {}", err);
            (500, "Internal server error\n".as_bytes().into())
        }
    };

    Ok(Response::builder()
        .status(status)
        .body(body.into())
        .expect("Status and body should be valid"))
}
