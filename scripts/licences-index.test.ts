// @vitest-environment happy-dom
import { existsSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { afterEach, describe, expect, it, vi } from "vitest";

// Paths, not file URLs: happy-dom replaces the global URL
const licences = join(dirname(fileURLToPath(import.meta.url)), "..", "public", "licenses");
const index = readFileSync(join(licences, "index.html"), "utf8");
const viewer = readFileSync(join(licences, "viewer.js"), "utf8");
/** Files the build writes into dist/licenses, beside public/licenses */
const BUILT = ["third-party-npm.html"];
const ships = (file: string) => BUILT.includes(file) || existsSync(join(licences, file));

const FILES: Record<string, string> = {
  "third-party-npm.html":
    '<!doctype html><html><head><meta name="aximar-notices" content="npm"></head><body><h1>npm packages: licence notices</h1><pre>react 19.0.0</pre></body></html>',
  "third-party-rust.html":
    '<!doctype html><html><head><meta name="aximar-notices" content="rust"></head><body><h1>Rust crates: licence notices</h1><p>457 crates</p></body></html>',
  "JetBrainsMono-OFL.txt": "Copyright 2020 The JetBrains Mono Project Authors",
  "ComputerModern-OFL.txt": "Computer Modern licence",
};

/** The page, with fetch answered from FILES, and the viewer started */
async function start(hash = "") {
  document.body.innerHTML = index.match(/<body>([\s\S]*)<\/body>/)![1].replace(/<script[\s\S]*?<\/script>/, "");
  history.replaceState(null, "", `/licenses/index.html${hash}`);
  vi.stubGlobal("fetch", async (file: string) => {
    const text = FILES[file];
    return { ok: text !== undefined, status: text === undefined ? 404 : 200, text: async () => text ?? "" };
  });
  const add = window.addEventListener.bind(window);
  vi.spyOn(window, "addEventListener").mockImplementation((type: string, listener: EventListenerOrEventListenerObject) => {
    listeners.push([type, listener]);
    add(type, listener);
  });
  new Function(viewer)();
  await (window as unknown as { __licencesShown: Promise<void> }).__licencesShown;
}

async function go(hash: string) {
  location.hash = hash;
  window.dispatchEvent(new HashChangeEvent("hashchange"));
  await new Promise((r) => setTimeout(r, 0));
  await new Promise((r) => setTimeout(r, 0));
}

const view = () => document.getElementById("view")!;
const current = () => document.querySelector('.bar a[aria-current="page"]')?.textContent;

/** The viewer's listeners, removed after each test: the window outlives it */
const listeners: [string, EventListenerOrEventListenerObject][] = [];

afterEach(() => {
  for (const [type, listener] of listeners.splice(0)) window.removeEventListener(type, listener);
  vi.restoreAllMocks();
  vi.unstubAllGlobals();
  document.body.innerHTML = "";
});

describe("the licences index", () => {
  it("shows every notice the app ships, and only those", () => {
    const files = [...viewer.matchAll(/file: "([^"]+)"/g)].map((m) => m[1]);
    expect(files.sort()).toEqual(Object.keys(FILES).sort());
    for (const f of files) expect(ships(f), f).toBe(true);
  });

  it("never links away from the viewer, where there'd be no way back", () => {
    const away = [...index.matchAll(/href="([^"]+)"/g)].map((m) => m[1]).filter((h) => !h.startsWith("#"));
    expect(away).toEqual([]);
  });
});

describe("the licences viewer", () => {
  it("opens on the overview", async () => {
    await start();
    expect(current()).toBe("Overview");
    expect(view().textContent).toContain("GNU General Public License");
  });

  it("shows a notice under the tabs, which stay to go back", async () => {
    await start();
    await go("#npm");
    expect(current()).toBe("npm packages");
    expect(view().querySelector("h1")?.textContent).toBe("npm packages: licence notices");
    expect(document.querySelectorAll(".bar nav a")).toHaveLength(4);
    await go("#overview");
    expect(current()).toBe("Overview");
    expect(view().textContent).toContain("GNU General Public License");
  });

  it("shows the Rust page's body, and both fonts", async () => {
    await start("#rust");
    expect(view().querySelector("h1")?.textContent).toBe("Rust crates: licence notices");
    await go("#fonts");
    expect([...view().querySelectorAll("h2")].map((h) => h.textContent)).toEqual(["JetBrains Mono", "Computer Modern Unicode"]);
  });

  it("says when a notice won't load", async () => {
    const saved = FILES["third-party-rust.html"];
    delete FILES["third-party-rust.html"];
    await start("#rust");
    expect(view().textContent).toContain("Couldn't load this notice");
    FILES["third-party-rust.html"] = saved;
  });

  it("explains, rather than shows, the app's page a development server sends for the npm notices", async () => {
    const saved = FILES["third-party-npm.html"];
    FILES["third-party-npm.html"] = '<!doctype html><html><head><script type="module" src="/src/main.tsx"></script></head><body><div id="root"></div></body></html>';
    await start("#npm");
    expect(view().textContent).toContain("written when the app is built");
    expect(view().textContent).not.toContain("<div");
    FILES["third-party-npm.html"] = saved;
  });

  it("refuses HTML where a text notice should be", async () => {
    const saved = FILES["JetBrainsMono-OFL.txt"];
    FILES["JetBrainsMono-OFL.txt"] = "<!doctype html><html></html>";
    await start("#fonts");
    expect(view().textContent).toContain("JetBrainsMono-OFL.txt is missing from this build");
    FILES["JetBrainsMono-OFL.txt"] = saved;
  });
});
