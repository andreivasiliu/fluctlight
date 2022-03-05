use std::panic::catch_unwind;

use fluctlight_mod_interface::{ModuleState, OpaqueModuleState, Request, ResponseResult};

mod canonical_hash;
mod matrix_types;
mod playground;
mod rendered_json;
mod request;
mod routes_admin;
mod routes_federation;
mod server_keys;
mod state;

#[no_mangle]
pub extern "C" fn process_request<'a>(request: Request<'a>) -> ResponseResult {
    eprintln!("Hello: {}", request.uri());

    let response = catch_unwind(|| {
        let state_box = &request.module_state().state;
        let state = state_box
            .downcast_ref::<state::State>()
            .expect("Unexpected kind of module state.");

        request::try_process_request(state, request)
    });

    response
        .map_err(|_err| "Handler panicked".to_string())
        .and_then(|response| response)
        .into()
}

#[no_mangle]
pub extern "C" fn create_state() -> OpaqueModuleState {
    let state = Box::new(state::State::new());

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
