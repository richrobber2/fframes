//! Hands [`fframes::live`] operations to the JS live runtime.

use fframes::live::LiveOps;
use wasm_bindgen::JsValue;

/// Encodes the operations as `[numbers: Uint32Array, strings: string, count: number]`, two copies
/// across the boundary however many operations there are (see [`LiveOps`] for the layout).
pub fn ops_to_js(ops: &LiveOps) -> JsValue {
    js_sys::Array::of3(
        &js_sys::Uint32Array::from(ops.numbers.as_slice()).into(),
        &JsValue::from_str(&ops.strings),
        &JsValue::from(ops.len() as u32),
    )
    .into()
}
