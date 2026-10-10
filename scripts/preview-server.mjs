import { spawn } from "node:child_process";
import { createReadStream } from "node:fs";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

// Only the example selected by the launcher can render. Requests cannot choose
// executables, filesystem paths, CLI flags, or another project.
export function draftExportPlugin(exampleDirectory, renderArgs = []) {
  let busy = false;
  let active;
  const stopActive = () => {
    if (active && active.exitCode === null) {
      try {
        process.kill(-active.pid, "SIGTERM");
      } catch {}
    }
  };
  return {
    name: "fframes-draft-export",
    configureServer(server) {
      server.httpServer?.once("close", stopActive);
      const terminate = () => {
        stopActive();
        process.exit(143);
      };
      process.once("SIGTERM", terminate);
      server.httpServer?.once("close", () =>
        process.removeListener("SIGTERM", terminate)
      );
      server.middlewares.use(async (req, res, next) => {
        if (
          req.url !== "/__fframes/capabilities" &&
          req.url !== "/__fframes/draft"
        )
          return next();
        const reject = (status, text) => {
          res.statusCode = status;
          res.setHeader("Content-Type", "text/plain");
          res.end(text);
        };
        if (
          req.headers["sec-fetch-site"] === "cross-site" ||
          (req.headers.origin &&
            req.headers.origin !== `http://${req.headers.host}`)
        )
          return reject(403, "Export must be requested from this preview.");
        if (req.url === "/__fframes/capabilities" && req.method === "GET") {
          res.setHeader("Content-Type", "application/json");
          return res.end(
            JSON.stringify({ draftExport: true, maxDurationSeconds: 30 })
          );
        }
        if (req.method !== "POST")
          return reject(405, "Use POST to export a draft.");
        if (!req.headers["content-type"]?.startsWith("application/json"))
          return reject(415, "Expected JSON.");
        if (busy) return reject(409, "A draft is already rendering.");
        let directory;
        let acquired = false;
        let child;
        const cancel = () => {
          if (child && child.exitCode === null) {
            try {
              process.kill(-child.pid, "SIGTERM");
            } catch {}
          }
        };
        try {
          let body = "";
          for await (const chunk of req) {
            body += chunk;
            if (body.length > 4096) return reject(413, "Request too large.");
          }
          const request = JSON.parse(body);
          if (!request || typeof request !== "object" || Array.isArray(request))
            return reject(400, "Expected a range object.");
          const { startSeconds, endSeconds } = request;
          if (
            typeof startSeconds !== "number" ||
            typeof endSeconds !== "number" ||
            !Number.isFinite(startSeconds) ||
            !Number.isFinite(endSeconds) ||
            startSeconds < 0 ||
            endSeconds <= startSeconds ||
            endSeconds - startSeconds > 30
          )
            return reject(400, "Select a valid range of up to 30 seconds.");
          // Recheck after reading the request, so concurrent uploads cannot race.
          if (busy) return reject(409, "A draft is already rendering.");
          busy = true;
          acquired = true;
          directory = await mkdtemp(join(tmpdir(), "fframes-draft-"));
          const output = join(directory, "draft.mp4");
          const runProcess = async (command, args) => {
            child = spawn(command, args, {
              cwd: exampleDirectory,
              detached: true,
              stdio: ["ignore", "ignore", "pipe"],
            });
            active = child;
            let log = "";
            child.stderr.on("data", chunk => {
              log = (log + chunk).slice(-8000);
            });
            await new Promise((resolve, reject) => {
              child.once("error", reject);
              child.once("exit", (code, signal) =>
                code === 0
                  ? resolve()
                  : reject(
                      new Error(`${command} failed (${code ?? signal}). ${log}`)
                    )
              );
            });
          };
          res.once("close", cancel);
          const timeout = setTimeout(cancel, 600000);
          try {
            // Native half-size rendering currently clips cached static SVG trees.
            // Preserve the scene at native size, then downscale the encoded draft.
            const native = join(directory, "native.mp4");
            await runProcess("cargo", [
              "run",
              "--locked",
              "--manifest-path",
              join(exampleDirectory, "Cargo.toml"),
              "--",
              ...renderArgs,
              "--scale",
              "1",
              "render",
              `${startSeconds}s..${endSeconds}s`,
              "--draft",
              "-o",
              native,
            ]);
            if (res.destroyed) return;
            await runProcess("ffmpeg", [
              "-nostdin",
              "-v",
              "error",
              "-i",
              native,
              "-vf",
              "scale=trunc(iw/4)*2:trunc(ih/4)*2",
              "-c:v",
              "libx264",
              "-preset",
              "ultrafast",
              "-crf",
              "30",
              "-c:a",
              "copy",
              "-movflags",
              "+faststart",
              "-y",
              output,
            ]);
          } finally {
            clearTimeout(timeout);
          }
          if (res.destroyed) return;
          res.setHeader("Content-Type", "video/mp4");
          res.setHeader(
            "Content-Disposition",
            'attachment; filename="draft.mp4"'
          );
          await new Promise((resolve, reject) => {
            const stream = createReadStream(output);
            stream.on("error", reject);
            res.once("finish", resolve);
            res.once("close", resolve);
            stream.pipe(res);
          });
        } catch (error) {
          if (!res.destroyed && !res.headersSent) {
            if (error instanceof SyntaxError)
              reject(400, "Invalid JSON request.");
            else {
              console.error(error.message);
              reject(
                500,
                "Draft rendering failed. Check the preview server terminal for details."
              );
            }
          }
        } finally {
          res.removeListener("close", cancel);
          if (acquired) {
            busy = false;
            active = undefined;
          }
          if (directory) await rm(directory, { recursive: true, force: true });
        }
      });
    },
  };
}
