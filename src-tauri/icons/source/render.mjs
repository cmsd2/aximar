// Render the icon SVGs to 1024px PNGs with Chromium.  tauri icon can read
// SVG itself, but its renderer drops the frame's drop shadow.
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const dir = fileURLToPath(new URL(".", import.meta.url));
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1024, height: 1024 } });
for (const name of ["master", "small", "master_win", "small_win"]) {
  const svg = readFileSync(dir + name + ".svg", "utf8");
  await page.setContent(`<html><body style="margin:0;background:transparent">${svg}</body></html>`);
  await page.screenshot({ path: dir + name + ".png", omitBackground: true, clip: { x: 0, y: 0, width: 1024, height: 1024 } });
}
await browser.close();
