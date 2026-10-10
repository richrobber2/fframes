#![cfg(target_arch = "wasm32")]
use fframes_editor_controller::prelude::*;
use hello_world_example::{HelloWorldMedia, HelloWorldVideo};

impl_wasm_bridge_for!(HelloWorldVideo<'static>, HelloWorldMedia);

lazy_static! {
    static ref MEDIA: HelloWorldMedia =
        HelloWorldMedia::prepare().expect("Failed static media processing for wasm bridge");
    static ref NAMES: Mutex<std::collections::HashSet<&'static str>> = Mutex::default();
}

/// Longest name the live demo accepts, in characters.
const MAX_NAME_CHARS: usize = 24;

/// The video borrows its name for `'static`, so every distinct name is leaked once and reused.
fn intern_name(name: &str) -> &'static str {
    let name: String = name.trim().chars().take(MAX_NAME_CHARS).collect();
    let mut names = NAMES.lock().unwrap();
    if let Some(existing) = names.get(name.as_str()) {
        return existing;
    }
    let leaked: &'static str = Box::leak(name.into_boxed_str());
    names.insert(leaked);
    leaked
}

#[wasm_bindgen]
pub fn create_wasm_bridge() -> WasmBridge {
    console_error_panic_hook::set_once();

    WasmBridge::new(
        HelloWorldVideo {
            media: &MEDIA,
            slug: "World",
        },
        &MEDIA,
    )
}

#[wasm_bindgen]
impl WasmBridge {
    /// Replaces the greeting ("Hello <name>"); used by the landing page's live demo.
    pub async fn set_name(&self, name: String) -> Result<VideoMetadata, JsValue> {
        let slug = match intern_name(&name) {
            "" => "World",
            slug => slug,
        };
        self.update_video(HelloWorldVideo {
            media: &MEDIA,
            slug,
        })
        .await
    }
}
