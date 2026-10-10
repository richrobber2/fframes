// The editor bridge imports its audio loader from "@fframes/editor". The landing demo has no
// audio, so the import map in index.html points that module here.
export async function load_audio_wasm_callback(path) {
  throw new Error(`The landing demo does not load audio (${path}).`);
}
