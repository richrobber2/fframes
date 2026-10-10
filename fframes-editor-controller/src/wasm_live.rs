//! Encodes [`fframes::live`] operations for the JS live runtime.

use fframes::live::{OP_REMOVE, OP_REPLACE, OP_SET, OP_TEXT, Op};
use wasm_bindgen::JsValue;

/// Encodes operations as `[[op, path, ...args], ...]` for the JS runtime.
pub fn ops_to_js(ops: &[Op]) -> JsValue {
    let array = js_sys::Array::new_with_length(ops.len() as u32);
    for (index, op) in ops.iter().enumerate() {
        let path = |path: &[u32]| -> JsValue { js_sys::Uint32Array::from(path).into() };
        let entry = match op {
            Op::Set {
                path: p,
                name,
                value,
            } => js_sys::Array::of4(&OP_SET.into(), &path(p), &name.into(), &value.into()),
            Op::Remove { path: p, name } => {
                js_sys::Array::of3(&OP_REMOVE.into(), &path(p), &name.into())
            }
            Op::Text { path: p, text } => {
                js_sys::Array::of3(&OP_TEXT.into(), &path(p), &text.into())
            }
            Op::Replace { path: p, markup } => {
                js_sys::Array::of3(&OP_REPLACE.into(), &path(p), &markup.into())
            }
        };
        array.set(index as u32, entry.into());
    }
    array.into()
}
