import { createServer } from "node:http";
import { readFile, stat } from "node:fs/promises";
import { existsSync, watch } from "node:fs";
import { spawn } from "node:child_process";
import { dirname, extname, join, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";

const project = dirname(fileURLToPath(import.meta.url));
const root = join(project, "live");
const port = Number(process.env.PORT || 5175);
const clients = new Set();
const reloadScript =
  '<script>new EventSource("/__reload").onmessage=()=>location.reload();</script>';
const mime = {
  ".html": "text/html; charset=utf-8",
  ".js": "text/javascript; charset=utf-8",
  ".css": "text/css; charset=utf-8",
  ".wasm": "application/wasm",
  ".ttf": "font/ttf",
  ".svg": "image/svg+xml",
};
if (!existsSync(join(root, "pkg/math_lab_bg.wasm"))) {
  console.error("Run node examples/math-lab/build-live.mjs first.");
  process.exit(1);
}
if (!Number.isInteger(port) || port < 1 || port > 65535)
  throw new Error("Invalid PORT.");

const server = createServer(async (req, res) => {
  try {
    const url = new URL(req.url, "http://localhost");
    if (url.pathname === "/__reload") {
      res.writeHead(200, {
        "Content-Type": "text/event-stream",
        "Cache-Control": "no-cache",
        Connection: "keep-alive",
      });
      res.write(": connected\n\n");
      clients.add(res);
      req.on("close", () => clients.delete(res));
      return;
    }
    const path = resolve(
      root,
      "." +
        decodeURIComponent(url.pathname === "/" ? "/index.html" : url.pathname)
    );
    if (!path.startsWith(root + sep) || !(await stat(path)).isFile()) {
      res.writeHead(404).end("Not found");
      return;
    }
    let body = await readFile(path);
    if (extname(path) === ".html")
      body = Buffer.from(
        body.toString().replace("</body>", reloadScript + "</body>")
      );
    res.writeHead(200, {
      "Content-Type": mime[extname(path)] || "application/octet-stream",
      "Cache-Control": "no-store",
    });
    res.end(body);
  } catch {
    res.writeHead(404).end("Not found");
  }
});
server.on("error", error => {
  console.error(error.message);
  process.exit(1);
});
server.listen(port, "0.0.0.0", () =>
  console.log(`Math in Motion · fframes live: http://localhost:${port}/`)
);

let timer,
  building = false,
  rebuildAgain = false,
  build;
const refresh = () => {
  for (const client of clients) client.write("data: reload\n\n");
};
function rebuild() {
  if (building) {
    rebuildAgain = true;
    return;
  }
  building = true;
  build = spawn(process.execPath, [join(project, "build-live.mjs")], {
    stdio: "inherit",
  });
  build.once("exit", code => {
    building = false;
    if (code === 0) refresh();
    else console.error("Rebuild failed; keeping the previous preview.");
    if (rebuildAgain) {
      rebuildAgain = false;
      rebuild();
    }
  });
}
const watchers = [
  watch(join(project, "src"), { recursive: true }, () => {
    clearTimeout(timer);
    timer = setTimeout(rebuild, 150);
  }),
  watch(
    join(project, "editor/editor-bridge"),
    { recursive: true },
    (_, file) => {
      if (file && /\.(rs|toml)$/.test(file)) {
        clearTimeout(timer);
        timer = setTimeout(rebuild, 150);
      }
    }
  ),
  watch(root, (_, file) => {
    if (file && /\.(html|js|css)$/.test(file) && !building) refresh();
  }),
];
for (const signal of ["SIGINT", "SIGTERM"])
  process.on(signal, () => {
    clearTimeout(timer);
    build?.kill();
    watchers.forEach(watcher => watcher.close());
    clients.forEach(client => client.end());
    server.close(() => process.exit(0));
  });
