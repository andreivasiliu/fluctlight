use std::{convert::Infallible, net::SocketAddr, path::PathBuf, sync::Arc};

use hyper::{
    server::conn::AddrIncoming,
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use libloading::library_filename;
use tls_listener::TlsListener;
use tokio_inotify::{AsyncINotify, IN_CREATE};
use tokio_rustls::{
    rustls::{Certificate, PrivateKey, ServerConfig},
    TlsAcceptor,
};

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

    eprintln!("Creating server at http://127.1.0.2:8008/admin/view");
    let addr = SocketAddr::from(([127, 1, 0, 2], 8008));

    let main_module = Arc::new(MainModule::new().unwrap());

    let main_module_2 = main_module.clone();

    tokio_runtime.spawn(watch_module(main_module.clone()));

    let make_server = async {
        eprintln!("Loading module...");

        let make_service = make_service_fn(move |_conn| {
            let main_module = main_module.clone();
            async {
                Ok::<_, Infallible>(service_fn(move |request: Request<Body>| {
                    let main_module = main_module.clone();
                    process_request_in_module(main_module, request)
                }))
            }
        });

        std::result::Result::<_, hyper::Error>::Ok(
            Server::try_bind(&addr)?
                .serve(make_service)
                .with_graceful_shutdown(shutdown_signal()),
        )
    };

    let http_server = tokio_runtime.block_on(make_server)?;
    eprintln!("HTTP server is ready.");

    let make_tls_server = async {
        let addr = SocketAddr::from(([127, 1, 0, 2], 8448));
        let acceptor: TlsAcceptor = {
            let key = PrivateKey(include_bytes!("../../key.der").as_slice().into());
            let cert = Certificate(include_bytes!("../../cert.der").as_slice().into());

            Arc::new(
                ServerConfig::builder()
                    .with_safe_defaults()
                    .with_no_client_auth()
                    .with_single_cert(vec![cert], key)
                    .unwrap(),
            )
            .into()
        };

        let incoming = TlsListener::new(acceptor, AddrIncoming::bind(&addr)?);

        let make_service = make_service_fn(move |_conn| {
            let main_module = main_module_2.clone();
            async {
                Ok::<_, Infallible>(service_fn(move |request: Request<Body>| {
                    let main_module = main_module.clone();
                    process_request_in_module(main_module, request)
                }))
            }
        });

        std::result::Result::<_, hyper::Error>::Ok(
            Server::builder(incoming)
                .serve(make_service)
                .with_graceful_shutdown(shutdown_signal()),
        )
    };
    let https_server = tokio_runtime.block_on(make_tls_server)?;
    eprintln!("HTTPS server is ready.");

    tokio_runtime.spawn(http_server);
    let handle = tokio_runtime.spawn(https_server);
    tokio_runtime.block_on(handle)??;

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
                eprintln!("Could not restart module on inotify event: {}", err);
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

async fn process_request_in_module(
    main_module: Arc<MainModule>,
    req: Request<Body>,
) -> std::result::Result<Response<Body>, Infallible> {
    let (parts, body) = req.into_parts();
    // FIXME: fix unwrap
    let body = hyper::body::to_bytes(body).await.unwrap().to_vec();
    let uri = parts.uri;
    let method = parts.method;

    let (status, content_type, body) = main_module.process_request(uri, method, body).await;

    Ok(Response::builder()
        .status(status)
        .header("Content-Type", content_type)
        .body(body.into())
        .expect("Status and body should be valid"))
}
