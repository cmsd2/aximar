#!/usr/bin/env node
// The translators' gallery: a screenshot of every Storybook story in every
// language, and which strings each story shows.
//
//   npm run build-storybook && node scripts/i18n-gallery.mjs
//
// Stories tagged `themes` are shot in the light and the dark theme, the
// rest in light; stories tagged `english-only` (the Foundations, whose
// text is token names) only in English.
//
// Writes storybook-static/gallery/ (screenshots and an index page), and
// locales/screens.json, which scripts/i18n-extract turns into
// `#. screen: <story>` comments in the catalog.  Needs Playwright's
// Chromium: `npx playwright install chromium`.  See docs/i18n.md.
import { createServer } from "node:http";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const root = fileURLToPath(new URL("..", import.meta.url));
const site = join(root, "storybook-static");
const gallery = join(site, "gallery");
const LANGUAGES = ["en", "es", "pseudo"];
const THEMES = ["light", "dark"];

/** The languages and themes to shoot a story in */
function shotsOf(story) {
  const tags = story.tags ?? [];
  const languages = tags.includes("english-only") ? ["en"] : LANGUAGES;
  const themes = tags.includes("themes") ? THEMES : ["light"];
  return languages.flatMap((lang) => themes.map((theme) => ({ lang, theme })));
}

/** A screenshot's file, under its language's folder */
function shotFile(story, theme) {
  return theme === "light" ? `${story.id}.png` : `${story.id}--${theme}.png`;
}

const TYPES = {
  ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".json": "application/json",
  ".svg": "image/svg+xml", ".png": "image/png", ".woff2": "font/woff2", ".woff": "font/woff", ".ttf": "font/ttf",
};

/** The static Storybook, served on a free port */
async function serve() {
  const server = createServer(async (req, res) => {
    const path = normalize(decodeURIComponent(new URL(req.url, "http://x").pathname)).replace(/^\/+/, "");
    try {
      const body = await readFile(join(site, path || "index.html"));
      res.writeHead(200, { "content-type": TYPES[extname(path)] ?? "application/octet-stream" });
      res.end(body);
    } catch {
      res.writeHead(404).end();
    }
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  return { server, base: `http://127.0.0.1:${server.address().port}` };
}

// The invisible tags of src/lib/show-sources.ts
const START = "⁣";
const END = "⁤";
function decodeIds(text) {
  const ids = [];
  for (const match of text.matchAll(new RegExp(`${START}([​‌]+)${END}`, "g"))) {
    let id = "";
    for (let i = 0; i + 8 <= match[1].length; i += 8) {
      let code = 0;
      for (const b of match[1].slice(i, i + 8)) code = (code << 1) | (b === "‌" ? 1 : 0);
      id += String.fromCodePoint(code);
    }
    ids.push(id);
  }
  return ids;
}

async function open(page, base, id, globals) {
  await page.goto(`${base}/iframe.html?id=${id}&viewMode=story&globals=${globals}`);
  try {
    await page.waitForSelector("[data-story-ready]", { timeout: 15000 });
    // Stories whose content arrives late mark themselves pending (src/stories/wait.tsx)
    await page.waitForFunction(() => !document.querySelector("[data-story-pending]"), null, { timeout: 15000 });
  } catch (e) {
    throw new Error(`story ${id} (${globals}) didn't finish loading: ${e.message}`);
  }
  await page.evaluate(() => document.fonts.ready);
  await page.waitForTimeout(150);
}

const { server, base } = await serve();
const browser = await chromium.launch();
const page = await browser.newPage({ viewport: { width: 1000, height: 1000 } });
try {
  const index = JSON.parse(await readFile(join(site, "index.json"), "utf8"));
  const stories = Object.values(index.entries).filter((e) => e.type === "story");

  // Screenshots, per language and theme.  Each shot also checks that no
  // SegmentedControl label wraps or overflows, as a long translation might.
  for (const lang of LANGUAGES) await mkdir(join(gallery, lang), { recursive: true });
  let shots = 0;
  const wrapped = [];
  let monoChecked = false;
  for (const story of stories) {
    for (const { lang, theme } of shotsOf(story)) {
      await open(page, base, story.id, `locale:${lang};theme:${theme}`);
      await page.screenshot({ path: join(gallery, lang, shotFile(story, theme)), fullPage: true });
      const bad = await page.evaluate(() =>
        [...document.querySelectorAll(".ui-segmented-option")]
          .filter((el) => {
            const lineHeight = parseFloat(getComputedStyle(el).lineHeight) || 20;
            return el.scrollWidth > el.clientWidth + 1 || el.getBoundingClientRect().height > lineHeight * 1.9;
          })
          .map((el) => el.textContent),
      );
      for (const label of bad) wrapped.push(`${story.id} (${lang}): "${label}"`);
      // The bundled monospace font: loaded, and what a code cell uses
      if (!monoChecked && story.id.startsWith("cells--")) {
        const mono = await page.evaluate(async () => {
          await document.fonts.ready;
          const code = document.querySelector(".cm-content");
          return {
            loaded: [...document.fonts].some((f) => f.family.replace(/"/g, "") === "JetBrains Mono" && f.status === "loaded"),
            family: code ? getComputedStyle(code).fontFamily : null,
          };
        });
        if (mono.family !== null) {
          if (!mono.loaded || !mono.family.startsWith('"JetBrains Mono"')) {
            throw new Error(`${story.id}: the code cell isn't in the bundled JetBrains Mono (${JSON.stringify(mono)})`);
          }
          monoChecked = true;
        }
      }
      shots++;
    }
  }
  if (!monoChecked) throw new Error("No Cells story showed a code cell to check the monospace font on");
  if (wrapped.length > 0) {
    throw new Error(`SegmentedControl labels that wrap or overflow:\n${wrapped.join("\n")}`);
  }

  // Which strings each story shows, from the tagged English
  const screens = {};
  for (const story of stories) {
    await open(page, base, story.id, "locale:en;tagStrings:on");
    const text = await page.evaluate(() => {
      const parts = [document.body.innerText];
      for (const el of document.querySelectorAll("[title],[placeholder],[alt],[aria-label]")) {
        for (const name of ["title", "placeholder", "alt", "aria-label"]) parts.push(el.getAttribute(name) ?? "");
      }
      return parts.join("\n");
    });
    for (const id of new Set(decodeIds(text))) (screens[id] ??= []).push(story.id);
  }
  const sorted = Object.fromEntries(
    Object.keys(screens).sort().map((id) => [id, [...new Set(screens[id])].sort()]),
  );
  await writeFile(join(root, "locales/screens.json"), JSON.stringify(sorted, null, 2) + "\n");

  // An index page of the screenshots
  const rows = stories
    .map((s) => {
      const shots = shotsOf(s)
        .map(({ lang, theme }) => {
          const file = `${lang}/${shotFile(s, theme)}`;
          const caption = theme === "light" ? lang : `${lang}, ${theme}`;
          return `<figure><figcaption>${caption}</figcaption><a href="${file}"><img src="${file}" loading="lazy"></a></figure>`;
        })
        .join("");
      return `<section id="${s.id}"><h2>${s.title} / ${s.name} <a href="../?path=/story/${s.id}">open</a></h2><div class="shots">${shots}</div></section>`;
    })
    .join("\n");
  await writeFile(
    join(gallery, "index.html"),
    `<!doctype html><meta charset="utf-8"><title>Aximar translation gallery</title>
<style>body{font-family:system-ui,sans-serif;margin:16px}.shots{display:flex;gap:12px;flex-wrap:wrap}figure{margin:0}img{max-width:420px;border:1px solid #ccc}</style>
<h1>Aximar translation gallery</h1><p>Each screen in English, Spanish and the pseudo-locale; the Foundations and components in both themes. <a href="../">Browse the stories</a>, with language and theme switches.</p>
${rows}\n`,
  );

  // Every story, in every language and theme it should have
  for (const story of stories) {
    for (const { lang, theme } of shotsOf(story)) await readFile(join(gallery, lang, shotFile(story, theme)));
  }
  console.log(`${shots} screenshots of ${stories.length} stories; ${Object.keys(sorted).length} strings placed on screens`);
} finally {
  await browser.close();
  server.close();
}
