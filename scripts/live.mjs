#!/usr/bin/env node
// `just live <example>`: builds the example's editor bridge for the browser and serves it as a
// live page (fframes-live/page.html), where the video runs interactively. With --check it opens
// the page in headless Chromium, renders frames across the timeline and exits non-zero on errors.
import { spawnSync } from "node:child_process";
import { copyFileSync, createReadStream, existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { createServer } from "node:http";
import { createRequire } from "node:module";
import { dirname, extname, join, normalize, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const { values, positionals } = parseArgs({
  allowPositionals: true,
  options: {
    port: { type: "string", default: "5180" },
    "skip-build": { type: "boolean", default: false },
    check: { type: "boolean", default: false },
    export: { type: "string" },
    help: { type: "boolean", default: false },
  },
});
const example = positionals[0];
if (values.help || !example) {
  console.log("Usage: just live <example> [--port 5180] [--skip-build] [--check] [--export <dir>]\n--export writes the static page (index.html, bridge, runtime, live.json) to <dir> instead of serving it.");
  process.exit(example ? 0 : 1);
}
if (!/^[a-zA-Z0-9_-]+$/.test(example)) throw new Error("Example must be a directory name under examples/.");

const exampleDir = join(root, "examples", example);
const bridgeDir = join(exampleDir, "editor", "editor-bridge");
if (!existsSync(join(bridgeDir, "Cargo.toml"))) {
  console.error(`examples/${example} has no editor bridge (editor/editor-bridge), so it cannot run live.`);
  process.exit(1);
}
// The [lib] name when it sets one, otherwise the package name with "-" turned into "_".
const manifest = readFileSync(join(bridgeDir, "Cargo.toml"), "utf8");
const section = (name) => manifest.split(/^\[/m).find((part) => part.startsWith(`${name}]`)) ?? "";
const nameIn = (text) => text.match(/^\s*name\s*=\s*"([^"]+)"/m)?.[1];
const libName = (nameIn(section("lib")) ?? nameIn(section("package")))?.replaceAll("-", "_");
if (!libName) throw new Error("Could not read the bridge's crate name from its Cargo.toml.");

const out = values.export ? resolve(root, values.export) : join(root, "target", "live", example);
function run(command, args, cwd) {
  const result = spawnSync(command, args, { cwd, stdio: "inherit" });
  if (result.error) throw new Error(`${command}: ${result.error.message}`);
  if (result.status !== 0) throw new Error(`${command} failed (${result.status ?? result.signal})`);
}

if (!values["skip-build"]) {
  run("cargo", ["build", "--lib", "--release", "--target", "wasm32-unknown-unknown"], bridgeDir);
  mkdirSync(out, { recursive: true });
  const wasm = join(root, "target", "wasm32-unknown-unknown", "release", `${libName}.wasm`);
  run("wasm-bindgen", ["--target", "web", "--no-typescript", "--out-dir", out, "--out-name", "bridge", wasm], root);
}
mkdirSync(out, { recursive: true });
copyFileSync(join(root, "fframes-live", "page.html"), join(out, "index.html"));
copyFileSync(join(root, "fframes-live", "fframes-live.js"), join(out, "fframes-live.js"));
copyFileSync(join(root, "fframes-live", "editor-shim.js"), join(out, "editor-shim.js"));
if (existsSync(join(exampleDir, "live.json"))) copyFileSync(join(exampleDir, "live.json"), join(out, "live.json"));
else writeFileSync(join(out, "live.json"), "{}\n");

if (values.export) {
  console.log(`examples/${example} exported to ${relative(root, out)}/`);
  process.exit(0);
}

const types = { ".html": "text/html", ".js": "text/javascript", ".wasm": "application/wasm", ".json": "application/json" };
const server = createServer((req, res) => {
  const path = decodeURIComponent(new URL(req.url, "http://localhost").pathname);
  const file = normalize(join(out, path === "/" ? "index.html" : path));
  if (relative(out, file).startsWith("..") || relative(out, file).includes(`..${sep}`) || !existsSync(file) || statSync(file).isDirectory()) {
    res.statusCode = 404;
    return res.end("Not found");
  }
  res.setHeader("Content-Type", types[extname(file)] ?? "application/octet-stream");
  res.setHeader("Cache-Control", "no-store");
  createReadStream(file).pipe(res);
});

server.listen(Number(values.port), "127.0.0.1", async () => {
  const url = `http://127.0.0.1:${values.port}/`;
  if (!values.check) {
    console.log(`examples/${example} live at ${url} (Ctrl+C to stop)`);
    return;
  }
  let code = 0;
  try {
    const require = createRequire(import.meta.url);
    const { chromium } = require(process.env.FFRAMES_PLAYWRIGHT_PATH || "playwright");
    const browser = await chromium.launch(process.env.FFRAMES_CHROMIUM_PATH ? { executablePath: process.env.FFRAMES_CHROMIUM_PATH } : {});
    const page = await browser.newPage({ viewport: { width: 1280, height: 900 } });
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    page.on("console", message => message.type() === "error" && errors.push(message.text()));
    await page.goto(url);
    await page.waitForFunction(() => ["ready", "error"].includes(document.querySelector(".panel").dataset.state), null, { timeout: 60000 });
    const state = await page.evaluate(() => document.querySelector(".panel").dataset.state);
    if (state === "ready") {
      const last = Number(await page.getAttribute(".scrub", "max"));
      for (const at of [0, 0.25, 0.5, 0.75, 1]) {
        await page.fill(".scrub", String(Math.round(last * at)));
        await page.waitForTimeout(80);
      }
      const box = await page.locator(".frame").boundingBox();
      await page.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
      await page.mouse.click(box.x + box.width / 2, box.y + box.height / 2);
      await page.waitForTimeout(200);
    }
    const elements = await page.evaluate(() => document.querySelectorAll(".frame *").length);
    console.log(`${example}: ${state}, ${elements} elements in the frame${errors.length ? `, errors:\n  ${errors.join("\n  ")}` : ""}`);
    if (state !== "ready" || errors.length || elements === 0) code = 1;
    await browser.close();
  } catch (error) {
    console.error(error);
    code = 1;
  }
  server.close();
  process.exit(code);
});
