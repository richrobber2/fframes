#![cfg(target_arch = "wasm32")]
use fframes_editor_controller::prelude::*;
use math_ideas_example::{MathIdeas, MathIdeasMedia};

impl_wasm_bridge_for!(MathIdeas<'static>, MathIdeasMedia);

lazy_static! {
    static ref MEDIA: MathIdeasMedia =
        MathIdeasMedia::prepare().expect("Failed static media processing for wasm bridge");
}

#[wasm_bindgen]
pub fn create_wasm_bridge() -> WasmBridge {
    console_error_panic_hook::set_once();
    WasmBridge::new(MathIdeas { media: &MEDIA }, &MEDIA)
}
