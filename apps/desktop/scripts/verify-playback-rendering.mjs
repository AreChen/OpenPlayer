import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.OPENPLAYER_PLAYWRIGHT_MODULE || "playwright");
const server = await createServer({ server: { port: 0, strictPort: false, open: false } });
await server.listen();
let browser;
const output = new URL("../../../target/playback-rendering/", import.meta.url);
await mkdir(output, { recursive: true });
try {
  browser = await chromium.launch({ headless: true, executablePath: process.env.OPENPLAYER_BROWSER_EXECUTABLE });
  for (const viewport of [{ width: 1280, height: 720 }, { width: 390, height: 844 }]) {
    const page = await browser.newPage({ viewport });
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.goto(`${server.resolvedUrls.local[0]}tests/browser/playback-clock.html`);
    await page.waitForFunction(() => window.clockTest?.clock);
    const before = await page.evaluate(() => ({ ...window.clockTest.metrics }));
    await page.waitForTimeout(1200);
    const active = await page.evaluate(() => ({ ...window.clockTest.metrics }));
    assert.equal(active.shellRenders, before.shellRenders, "clock frames must not render the shell");
    assert.ok(active.timelineCommits - before.timelineCommits > 10, "timeline should animate smoothly");
    assert.ok(Number(await page.locator(".seek-slider").inputValue()) > 11);
    await page.evaluate(() => window.clockTest.setVisible(false));
    await page.waitForTimeout(100);
    const hidden = await page.evaluate(() => ({ ...window.clockTest.metrics }));
    await page.waitForTimeout(500);
    const hiddenAfter = await page.evaluate(() => ({ ...window.clockTest.metrics }));
    assert.equal(hiddenAfter.timelineCommits, hidden.timelineCommits, "hidden timeline should not render");
    assert.equal(hiddenAfter.scheduledFrames, hidden.scheduledFrames, "hidden timeline should not schedule animation frames");
    await page.evaluate(() => { window.clockTest.setVisible(true); window.clockTest.clock.anchor(42, false, 120, 1); });
    await page.waitForFunction(() => document.querySelector(".seek-slider").value === "42");
    await page.getByRole("button", { name: "Current time" }).click();
    await page.waitForFunction(() => document.querySelector(".transport-time").textContent === "1,260");
    await page.screenshot({ path: fileURLToPath(new URL(`timeline-${viewport.width}.png`, output)) });
    assert.deepEqual(errors, []);
    console.log(JSON.stringify({ viewport, shellRendersDuringPlayback: active.shellRenders - before.shellRenders, timelineCommits: active.timelineCommits - before.timelineCommits, timelineRenderMs: active.timelineDurationMs - before.timelineDurationMs, hiddenFrameRequests: hiddenAfter.scheduledFrames - hidden.scheduledFrames }));
    await page.close();
  }
} finally {
  await browser?.close();
  await server.close();
}
