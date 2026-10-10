// Optional browser validation; no browser policy bypass or manual audio resume.
const { chromium } = require(
  process.env.FFRAMES_PLAYWRIGHT_PATH || "playwright"
);
(async () => {
  const browser = await chromium.launch({
    ...(process.env.FFRAMES_CHROMIUM_PATH
      ? { executablePath: process.env.FFRAMES_CHROMIUM_PATH }
      : {}),
    args: ["--disable-dev-shm-usage"],
  });
  try {
    const page = await browser.newPage({
      viewport: { width: 1440, height: 1000 },
    });
    const errors = [];
    page.on("pageerror", error => errors.push(error.message));
    await page.goto(process.argv[2], { waitUntil: "networkidle" });
    await page.getByRole("button", { name: "Play", exact: true }).click();
    await page.getByRole("button", { name: "Pause", exact: true }).waitFor();
    // The timeline cursor must actually move, rather than only toggling a button.
    const readTime = () =>
      page
        .locator("body")
        .innerText()
        .then(text => {
          const time = text.match(/\b(\d{2}:\d{2})\s*\/\s*\d{2}:\d{2}/)?.[1];
          if (!time)
            throw new Error("Playback time is missing from the editor.");
          return time;
        });
    const initial = await readTime();
    await page.waitForFunction(
      initial => {
        const time = document.body.innerText.match(
          /\b(\d{2}:\d{2})\s*\/\s*\d{2}:\d{2}/
        )?.[1];
        return time !== undefined && time !== initial;
      },
      initial,
      { timeout: 10000 }
    );
    await page.getByRole("button", { name: "Pause", exact: true }).click();
    await page.getByRole("button", { name: "Play", exact: true }).waitFor();
    const beforeSeek = await readTime();
    await page.getByRole("button", { name: "Forward 2s", exact: true }).click();
    const paused = await readTime();
    if (paused === beforeSeek)
      throw new Error("Forward seek did not change the playback time.");
    await page.waitForTimeout(350);
    if ((await readTime()) !== paused)
      throw new Error("Paused preview continued advancing after seeking.");
    await page.getByRole("button", { name: "Play", exact: true }).click();
    await page.waitForFunction(
      initial => {
        const time = document.body.innerText.match(
          /\b(\d{2}:\d{2})\s*\/\s*\d{2}:\d{2}/
        )?.[1];
        return time !== undefined && time !== initial;
      },
      paused,
      { timeout: 10000 }
    );
    await page.getByRole("button", { name: "Pause", exact: true }).click();
    if (errors.length) throw new Error(errors.join("\n"));
    console.log(
      "Browser check passed: playback advances, pause holds, seeking and replay work."
    );
  } finally {
    await browser.close();
  }
})().catch(error => {
  console.error(error.message);
  process.exitCode = 1;
});
