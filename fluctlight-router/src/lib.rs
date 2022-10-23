use std::panic::catch_unwind;

use fluctlight_mod_interface::{ModuleState, OpaqueModuleState, Request, ResponseResult, Response};

mod canonical_hash;
mod edu_ref;
mod interner;
mod matrix_types;
mod net_log;
mod pdu_arc;
mod pdu_owned;
mod pdu_ref;
mod persistence;
mod playground;
mod rendered_json;
mod request;
mod routes_admin;
mod routes_federation;
mod server_keys;
mod signed_request;
mod state;

use cap::Cap;
use playground::load_persistent_rooms;

#[global_allocator]
static ALLOCATOR: Cap<std::alloc::System> = Cap::new(std::alloc::System, usize::max_value());

#[no_mangle]
pub extern "C" fn process_request<'a>(request: Request<'a>) -> ResponseResult {
    eprintln!("{} {}", request.method(), request.uri());

    let response = catch_unwind(|| {
        let state_box = &request.module_state().state;
        let state = state_box
            .downcast_ref::<state::State>()
            .expect("Unexpected kind of module state.");

        request::try_process_request(state, request)
    });

    let response = match response {
        Ok(response) => response,
        Err(_panic_payload) => {
            Response::new(
                500,
                "text/plain",
                "Internal server error (request handler panicked)".as_bytes().into(),
            )
        },
    };

    if response.status() != 200 {
        eprintln!("Response: {}", response.status());
    }

    // TODO: Ok() and ResponseResult might no longer be needed
    Ok(response).into()
}

#[no_mangle]
pub extern "C" fn create_state() -> OpaqueModuleState {
    let state = Box::new(state::State::new());

    // println!("Usage before: {}MB", ALLOCATOR.allocated() / 1024 / 1024);
    // load_room(&state).expect("Could not load state.");
    // println!("Usage after: {}MB", ALLOCATOR.allocated() / 1024 / 1024);

    load_persistent_rooms(&state);

    let module_state = ModuleState { state };

    module_state.into_opaque()
}

// TODO: improper_ctypes_definitions complains about the () from RBox<()>, which
// is FFI-safe. This needs an issue on abi_stable's crate.
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn destroy_state(module_state: OpaqueModuleState) -> bool {
    let result = catch_unwind(|| {
        let state = ModuleState::from_opaque(module_state);
        drop(state.state);
    });

    match result {
        Ok(()) => true,
        Err(_err) => {
            eprintln!("Failed to destroy module state");
            false
        }
    }
}
