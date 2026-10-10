#![cfg(target_arch = "wasm32")]

use fframes_editor_controller::prelude::*;
use math_lab::{Lab, MathLab, MathLabMedia};
use std::sync::LazyLock;

impl_wasm_bridge_for!(MathLab, MathLabMedia);

static MEDIA: LazyLock<MathLabMedia> =
    LazyLock::new(|| MathLabMedia::prepare().expect("The bundled math-lab font must load"));

/// Create one independent live scene with its own input and SVG diff state.
#[wasm_bindgen]
pub fn create_wasm_bridge(mode: &str) -> WasmBridge {
    console_error_panic_hook::set_once();
    WasmBridge::new(MathLab::new(Lab::from_name(mode)), &MEDIA)
}
