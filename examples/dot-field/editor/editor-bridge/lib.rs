#![cfg(target_arch = "wasm32")]
use dot_field_example::{DotField, DotFieldMedia};
use fframes_editor_controller::prelude::*;

impl_wasm_bridge_for!(DotField<'static>, DotFieldMedia);

lazy_static! {
    static ref MEDIA: DotFieldMedia =
        DotFieldMedia::prepare().expect("Failed static media processing for wasm bridge");
}

#[wasm_bindgen]
pub fn create_wasm_bridge() -> WasmBridge {
    console_error_panic_hook::set_once();
    WasmBridge::new(DotField::new(&MEDIA), &MEDIA)
}
