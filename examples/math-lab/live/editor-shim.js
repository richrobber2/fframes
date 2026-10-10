// The official bridge imports this callback; these experiments have no audio.
export async function load_audio_wasm_callback(path) {
  throw new Error(`Math in Motion does not load audio (${path}).`);
}
