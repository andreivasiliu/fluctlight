use std::{convert::Infallible, net::SocketAddr, sync::Arc};

use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use smallvec::SmallVec;

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

    let make_server = async {
        eprintln!("Loading module...");
        let main_module = Arc::new(MainModule::new().unwrap());

        let make_service = make_service_fn(move |_conn| {
            let main_module = main_module.clone();
            async {
                Ok::<_, Infallible>(service_fn(move |request: Request<Body>| {
                    let main_module = main_module.clone();
                    hello_world(main_module, request)
                }))
            }
        });

        std::result::Result::<_, hyper::Error>::Ok(Server::try_bind(&addr)?.serve(make_service))
    };

    let server = tokio_runtime.block_on(make_server)?;

    eprintln!("Blocking...");
    tokio_runtime.block_on(server)?;

    Ok(())
}

async fn hello_world(
    main_module: Arc<MainModule>,
    req: Request<Body>,
) -> std::result::Result<Response<Body>, Infallible> {
    let (status, body) = main_module.process_request(req.uri().path()).await;

    let mut uri_segments: SmallVec<[&str; 8]> = req.uri().path().split('/').collect();
    uri_segments[0] = req.method().as_str();

    // match uri_segments.as_slice() {
    //     ["GET", "_matrix", "federation", "v1", "version"] => Ok(Response::new("Version!".into())),
    //     _ => Ok(Response::new("Hello, World".into())),
    // };

    Ok(Response::builder()
        .status(status)
        .body(body.into())
        .expect("Status and body should be valid"))
}
