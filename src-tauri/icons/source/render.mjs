// Render master.svg and small.svg to 1024px PNGs with Chromium.  tauri icon
// can read SVG itself, but its renderer drops the frame's drop shadow.
import { readFileSync } from "node:fs";
import { chromium } from "playwright";

const dir = new URL(".", import.meta.url).pathname;
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1024, height: 1024 } });
for (const name of ["master", "small"]) {
  const svg = readFileSync(dir + name + ".svg", "utf8");
  await page.setContent(`<html><body style="margin:0;background:transparent">${svg}</body></html>`);
  await page.screenshot({ path: dir + name + ".png", omitBackground: true, clip: { x: 0, y: 0, width: 1024, height: 1024 } });
}
await browser.close();
