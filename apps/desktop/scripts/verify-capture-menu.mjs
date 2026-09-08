import assert from "node:assert/strict";
import { createRequire } from "node:module";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

const require = createRequire(import.meta.url);
const { chromium } = require(process.env.OPENPLAYER_PLAYWRIGHT_MODULE || "playwright");
const server = await createServer({ server: { port: 0, open: false } });
await server.listen();
let browser;
const output = new URL("../../../target/capture-menu/", import.meta.url);
await mkdir(output, { recursive: true });
try {
  browser = await chromium.launch({ headless: true, executablePath: process.env.OPENPLAYER_BROWSER_EXECUTABLE });
  for (const viewport of [{ width: 1280, height: 720 }, { width: 960, height: 540 }, { width: 390, height: 844 }]) {
    for (const locale of ["en-US", "zh-CN"]) {
      const page = await browser.newPage({ viewport });
      const errors = [];
      page.on("pageerror", (error) => errors.push(error.message));
      await page.goto(`${server.resolvedUrls.local[0]}tests/browser/capture-menu.html?locale=${locale}`);
      const capture = page.getByRole("menuitem", { name: locale === "zh-CN" ? "外部增强模式" : "External Capture", exact: true });
      await capture.waitFor();
      const bounds = await page.getByRole("menu").boundingBox();
      assert.ok(bounds.x >= 0 && bounds.y >= 0 && bounds.x + bounds.width <= viewport.width && bounds.y + bounds.height <= viewport.height);
      assert.equal(await capture.locator("span").evaluate((element) => element.scrollWidth <= element.clientWidth), true);
      await capture.click();
      assert.equal(await page.locator("html").getAttribute("data-capture"), "entered");
      await page.screenshot({ path: fileURLToPath(new URL(`${locale}-${viewport.width}.png`, output)) });
      assert.deepEqual(errors, []);
      console.log(`PASS: capture menu ${locale} ${viewport.width}x${viewport.height}`);
      await page.close();
    }
  }
} finally {
  await browser?.close();
  await server.close();
}
