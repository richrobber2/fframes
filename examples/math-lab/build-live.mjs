import { spawnSync } from "node:child_process";
import { copyFileSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const project = dirname(fileURLToPath(import.meta.url));
const root = resolve(project, "../..");
const output = join(project, "live/pkg");
mkdirSync(output, { recursive: true });

function run(command, args) {
  const result = spawnSync(command, args, { cwd: root, stdio: "inherit" });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}

run("cargo", [
  "build",
  "--release",
  "--locked",
  "-p",
  "math-lab-bridge",
  "--target",
  "wasm32-unknown-unknown",
]);
run("wasm-bindgen", [
  "--target",
  "web",
  "--no-typescript",
  "--out-dir",
  output,
  "--out-name",
  "math_lab",
  join(root, "target/wasm32-unknown-unknown/release/math_lab_bridge.wasm"),
]);
copyFileSync(
  join(root, "fframes-live/fframes-live.js"),
  join(project, "live/fframes-live.js")
);
console.log("Built math-lab with the repository's fframes-live runtime.");
