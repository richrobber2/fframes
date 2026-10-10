# Math in Motion — fframes live

Three interactive Rust scenes: Fourier series, linear transformations, and the logistic map. The browser runs their WebAssembly bridge through `fframes-live.mount`. Controls reach Rust through `ctx.input()`, and `render_live` patches persistent SVG elements. There is no canvas renderer or hosted service.

From the repository root, with Rust, the `wasm32-unknown-unknown` target, and the workspace-compatible `wasm-bindgen` CLI available:

```sh
node examples/math-lab/build-live.mjs
node examples/math-lab/dev.mjs
```

Open `http://localhost:5175`. Rust edits rebuild the WebAssembly module and refresh the preview. HTML, CSS, and JavaScript edits refresh the preview directly. Set `PORT` to change the port.

In the provided cloud workspace, load its existing compiler configuration before these commands:

```sh
source /workspace/.setup/env.sh
```

The same scenes support the standard fframes CLI:

```sh
cargo run --release -p math-lab -- --lab fourier timeline
cargo run --release -p math-lab -- --lab linear inspect
cargo run --release -p math-lab -- --lab chaos frame 0s
```

The example's CLI uses the CPU backend for offline SVG inspection and frame review. The interactive preview uses the live browser runtime. The scene includes the DM Sans font, and all preview assets are local.

Fourier uses a 32-second loop so every supported quarter-step playback speed completes whole cycles. Matrix animation uses a cached fframes timeline. The bifurcation cloud is computed once for each chaos scene, with 601 rates and 48 retained samples after 1,200 discarded iterations; it is a finite approximation of long-run behavior.
