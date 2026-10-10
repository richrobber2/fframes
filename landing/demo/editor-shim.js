// The editor bridge imports its audio loader from "@fframes/editor". Pages without the editor
// map that module here with an import map. Audio embedded with `include_media_dir!` never reaches
// this loader; only audio the video expects the page to fetch does, and live pages do not.
export async function load_audio_wasm_callback(path) {
  throw new Error(`This live page does not load audio files (${path}); embed it with include_media_dir!.`);
}
