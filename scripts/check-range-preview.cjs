// Integration check for the hello-world example; requires the local preview launcher.
const { chromium } = require(
  process.env.FFRAMES_PLAYWRIGHT_PATH || "playwright"
);
(async () => {
  const b = await chromium.launch({
    ...(process.env.FFRAMES_CHROMIUM_PATH
      ? { executablePath: process.env.FFRAMES_CHROMIUM_PATH }
      : {}),
  });
  try {
    const p = await b.newPage({ viewport: { width: 1440, height: 1000 } });
    let errors = [];
    p.on("pageerror", e => errors.push(e.message));
    await p.goto(process.argv[2] + "/", { waitUntil: "networkidle" });
    await p.getByLabel("Range end seconds").fill("0");
    if (Number(await p.getByLabel("Range end seconds").inputValue()) <= 0)
      throw Error("Empty range accepted.");
    await p.getByLabel("Range end seconds").fill("30");
    await p.getByLabel("Range start seconds").fill("2");
    await p.getByLabel("Range end seconds").fill("3");
    await p
      .getByRole("button", { name: "Loop range off", exact: true })
      .click();
    await p.getByRole("button", { name: "Play", exact: true }).click();
    const frames = [];
    for (let i = 0; i < 25; i++) {
      frames.push(
        await p
          .locator("svg text")
          .filter({ hasText: "This frame index:" })
          .first()
          .textContent()
      );
      await p.waitForTimeout(100);
    }
    await p.getByRole("button", { name: "Pause", exact: true }).click();
    const numbers = frames.map(t => Number(t.match(/index: (\d+)/)[1]));
    if (
      numbers.some(n => n < 60 || n >= 90) ||
      !numbers.some((n, i) => i > 0 && n < numbers[i - 1])
    )
      throw Error(
        "Loop did not remain inside selected range and wrap: " + numbers
      );
    await p.screenshot({
      path:
        process.env.FFRAMES_SCREENSHOT_PATH ||
        require("node:path").join(
          require("node:os").tmpdir(),
          "fframes-range-loop.png"
        ),
    });
    await p.getByRole("button", { name: "Loop range on", exact: true }).click();
    await p.getByRole("button", { name: "Play", exact: true }).click();
    await p.waitForFunction(() =>
      [...document.querySelectorAll("svg text")].some(
        e => Number(e.textContent.match(/index: (\d+)/)?.[1]) >= 90
      )
    );
    await p.getByRole("button", { name: "Pause", exact: true }).click();
    await p.getByLabel("Range start seconds").fill("0");
    await p.getByLabel("Range end seconds").fill("1");
    console.log(
      "Range loop wraps repeatedly; disabling it resumes ordinary playback.",
      numbers
    );
    const response = await p.request.post(
      process.argv[2] + "/__fframes/draft",
      { data: { startSeconds: 0, endSeconds: 31 } }
    );
    if (response.status() !== 400) throw Error("Oversized export accepted");
    const cross = await p.request.post(process.argv[2] + "/__fframes/draft", {
      headers: { Origin: "https://example.com" },
      data: { startSeconds: 0, endSeconds: 1 },
    });
    if (cross.status() !== 403) throw Error("Cross-origin export accepted");
    const downloaded = p.waitForEvent("download", { timeout: 600000 });
    await p
      .getByRole("button", { name: "Export draft MP4", exact: true })
      .click();
    console.log("Draft render requested.");
    const d = await Promise.race([
      downloaded,
      p
        .getByRole("status")
        .filter({ hasText: /failed/i })
        .waitFor({ timeout: 600000 })
        .then(() => {
          throw Error("Draft rendering failed; inspect server logs.");
        }),
    ]);
    await d.saveAs(
      process.env.FFRAMES_DRAFT_PATH ||
        require("node:path").join(
          require("node:os").tmpdir(),
          "fframes-draft.mp4"
        )
    );
    if (await d.failure()) throw Error(await d.failure());
    if (errors.length) throw Error(errors.join("\n"));
    console.log("MP4 downloaded successfully; no page errors.");
  } finally {
    await b.close();
  }
})().catch(e => {
  console.error(e);
  process.exitCode = 1;
});
