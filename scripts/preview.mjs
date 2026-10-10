#!/usr/bin/env node
import { spawn, spawnSync } from "node:child_process";
import {
  existsSync,
  mkdtempSync,
  readdirSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { createServer } from "node:net";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const { values } = parseArgs({
  options: {
    example: { type: "string", default: "hello-world" },
    "render-arg": { type: "string", multiple: true },
    port: { type: "string", default: "5174" },
    "skip-build": { type: "boolean", default: false },
    check: { type: "boolean", default: false },
    help: { type: "boolean", default: false },
  },
});
if (values.help) {
  console.log(
    "Usage: node scripts/preview.mjs [--example hello-world] [--port 5174] [--skip-build] [--check] [--render-arg=ARG]\nBuild and serve an example. --check verifies Chromium playback and exits.\nRequires pnpm, Rust, wasm-pack and matching wasm-bindgen; --check also requires Playwright."
  );
  process.exit(0);
}
let child;
let temporary;
function cleanup() {
  if (child && child.exitCode === null && child.signalCode === null) {
    if (process.platform === "win32") child.kill();
    else {
      try {
        process.kill(-child.pid, "SIGTERM");
      } catch (error) {
        if (error.code !== "ESRCH") throw error;
      }
    }
  }
  if (temporary) rmSync(temporary, { recursive: true, force: true });
}
process.on("exit", cleanup);
for (const signal of ["SIGINT", "SIGTERM"])
  process.on(signal, () => process.exit(signal === "SIGINT" ? 130 : 143));
const pnpm = process.platform === "win32" ? "pnpm.cmd" : "pnpm";
function run(command, args, cwd) {
  const result = spawnSync(command, args, { cwd, stdio: "inherit" });
  if (result.error) throw new Error(`${command}: ${result.error.message}`);
  if (result.status !== 0)
    throw new Error(`${command} failed (${result.status ?? result.signal})`);
}
async function main() {
  if (!/^[a-zA-Z0-9_-]+$/.test(values.example))
    throw new Error("Example must be a directory name under examples/.");
  const port = Number(values.port);
  if (!Number.isInteger(port) || port < 1 || port > 65535)
    throw new Error("Port must be an integer from 1 to 65535.");
  const renderArgs =
    values["render-arg"] ??
    (values.example === "hello-world" ? ["--slug", "World"] : []);
  const editor = join(root, "examples", values.example, "editor");
  const bridge = join(editor, "editor-bridge");
  const config = join(editor, "vite.config.ts");
  if (!existsSync(config) || !existsSync(join(bridge, "Cargo.toml")))
    throw new Error(
      `Example ${values.example} has no supported editor/bridge.`
    );
  // Fail before building rather than connecting to someone else's server.
  await new Promise((resolve, reject) => {
    const socket = createServer();
    socket.once("error", () =>
      reject(
        new Error(
          `Port ${port} is unavailable; choose --port or stop your existing preview.`
        )
      )
    );
    socket.listen(port, "127.0.0.1", () => socket.close(resolve));
  });
  if (!values["skip-build"]) {
    run(pnpm, ["--dir", "fframes-editor", "build"], root);
    run(
      "wasm-pack",
      ["build", "--target", "web", "--dev", "--locked", "--mode", "no-install"],
      bridge
    );
  }
  const pkg = join(bridge, "pkg");
  if (!existsSync(pkg))
    throw new Error("Missing WASM output; rerun without --skip-build.");
  const wasm = readdirSync(pkg).find(name => name.endsWith("_bg.wasm"));
  if (!wasm)
    throw new Error("Missing WASM binary; rerun without --skip-build.");
  temporary = mkdtempSync(join(tmpdir(), "fframes-preview-"));
  const localConfig = join(temporary, "vite.config.mjs");
  // Resolve React from its declared editor dependency, without relying on hoisting.
  writeFileSync(
    localConfig,
    `import {createRequire} from 'node:module';\nimport {draftExportPlugin} from ${JSON.stringify(join(root, "scripts/preview-server.mjs"))};\nimport original from ${JSON.stringify(config)};\nconst editorRequire=createRequire(${JSON.stringify(join(root, "fframes-editor/package.json"))});\nconst exampleRequire=createRequire(${JSON.stringify(join(editor, "package.json"))});\nconst {mergeConfig}=exampleRequire('vite');\nconst folder=name=>editorRequire.resolve(name+'/package.json').replace(/\\/package.json$/, '');\nexport default mergeConfig(original,{plugins:[draftExportPlugin(${JSON.stringify(join(editor, ".."))},${JSON.stringify(renderArgs)})],resolve:{alias:{react:folder('react'),'react-dom':folder('react-dom')}}});\n`
  );
  child = spawn(
    pnpm,
    [
      "exec",
      "vite",
      "--config",
      localConfig,
      "--host",
      "127.0.0.1",
      "--port",
      String(port),
      "--strictPort",
    ],
    { cwd: editor, stdio: "inherit", detached: process.platform !== "win32" }
  );
  let failure;
  child.on("error", error => {
    failure = error;
  });
  child.on("exit", (code, signal) => {
    failure = new Error(`Preview stopped (${code ?? signal}).`);
  });
  const base = `http://127.0.0.1:${port}`;
  const deadline = Date.now() + 30000;
  let ready = false;
  while (Date.now() < deadline) {
    if (failure) throw failure;
    try {
      for (const path of ["/", "/main.tsx", `/editor-bridge/pkg/${wasm}`]) {
        const response = await fetch(base + path, {
          signal: AbortSignal.timeout(2000),
        });
        if (!response.ok)
          throw new Error(`HTTP ${response.status} for ${path}`);
        if (path.endsWith(".wasm")) {
          const bytes = new Uint8Array(await response.arrayBuffer());
          if (
            bytes[0] !== 0 ||
            bytes[1] !== 97 ||
            bytes[2] !== 115 ||
            bytes[3] !== 109
          )
            throw new Error("Invalid WASM response.");
        } else await response.text();
      }
      ready = true;
      break;
    } catch {
      await new Promise(resolve => setTimeout(resolve, 250));
    }
  }
  if (!ready)
    throw new Error(
      "Preview did not become ready within 30 seconds; inspect the Vite errors above."
    );
  console.log(`Preview assets ready for ${values.example} on port ${port}.`);
  if (values.check) {
    run(
      process.execPath,
      [join(root, "scripts/check-preview.cjs"), base],
      root
    );
    return;
  }
  console.log(
    "Press Ctrl+C to stop. Run with --check to verify actual browser playback."
  );
  await new Promise((resolve, reject) => {
    if (failure) reject(failure);
    child.once("error", reject);
    child.once("exit", code =>
      code === 0 ? resolve() : reject(new Error(`Vite exited (${code}).`))
    );
  });
}
main()
  .then(() => process.exit(0))
  .catch(error => {
    console.error(error.message);
    process.exit(1);
  });
