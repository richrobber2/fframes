# Editor module test tool

From a configured fframes checkout (Rust wasm32 target, wasm-pack, native dependencies and pnpm installed):

```sh
pnpm install
node tools/editor-tests/build.mjs
node tools/editor-tests/serve.mjs
```

Open http://localhost:4173 and choose **Run module tests**. The build prepares missing WASM packages, builds the editor and all three players, and assembles the shell. Generated bundles are ignored. No Sites account or credentials are needed.

The browser tool measures eight modules across three examples and six sizes. **Size test → Animate sizes** enables continuous squeeze/stretch testing during playback. Download the JSON report to inspect per-module failures. Hidden modules are skipped. Testing pauses playback and restores the original example at normal viewport size.

Agent commands from the repository root:

```sh
node tools/editor-tests/scripts/test-detector.mjs
node tools/editor-tests/scripts/module-tests.mjs --repo . --out module-report.json
node tools/editor-tests/scripts/module-tests.mjs --repo . --browser-report /path/to/fframes-module-report.json
```

Browser automation can invoke `await window.fframesTests.run()`, `.cancel()`, `.getReport()` and `.download()`. Code checks include 1080 compiled layout cases and production asset references. The result remains partial until a complete passing browser report is supplied. Geometry is not an assertion of visual quality, audio, keyboard accessibility or all animation frames.

Validated here: editor and all Vite builds, layout matrix, detector fixtures, and scene-output sampling. Real-browser execution remains unverified. Rust clippy is blocked by pre-existing fframes-media warnings with the installed Rust toolchain.
