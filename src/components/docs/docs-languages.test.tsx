// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));

import { DocsPanel } from "../DocsPanel";
import { answer, calls, click, mount, settle, unmount } from "../settings/settings-test-kit";
import { act } from "react";
import type { DocsLanguages, MaximaFunction } from "../../types/catalog";

const SPANISH_SUMMARIES: DocsLanguages = {
  choice: null,
  effective: "es",
  languages: [
    { tag: "en", full: true, pack: null },
    { tag: "es", full: false, pack: null },
  ],
};

function integrate(languages: MaximaFunction["languages"]): MaximaFunction {
  return {
    name: "integrate",
    signatures: ["integrate(expr, x)"],
    description: "The full English documentation.",
    category: "Calculus",
    examples: [],
    see_also: [],
    languages,
  };
}

async function open(fn: MaximaFunction, info: DocsLanguages = SPANISH_SUMMARIES) {
  const saved: unknown[] = [];
  answer({
    docs_languages: info,
    get_function: fn,
    get_function_docs: "The full English documentation.",
    set_config: ({ updates }: Record<string, unknown>) => {
      saved.push(updates);
      return null;
    },
  });
  const container = await mount(<DocsPanel open functionName="integrate" requestId={1} onClose={() => {}} />);
  await settle();
  return { container, saved };
}

const status = (container: HTMLElement, name: string) => container.querySelector(`[data-status="${name}"]`);

afterEach(() => {
  unmount();
  vi.restoreAllMocks();
});

describe("the docs panel's language", () => {
  it("offers Same as Aximar and each language with documentation, by its own name", async () => {
    const { container } = await open(integrate(undefined));
    const options = [...container.querySelectorAll(".docs-language-menu option")].map((o) => o.textContent);
    expect(options).toEqual(["Same as Aximar", "English", "Español"]);
    expect(container.querySelector<HTMLSelectElement>(".docs-language-menu select")?.value).toBe("same");
  });

  it("saves a choice, and Same as Aximar as null", async () => {
    const { container, saved } = await open(integrate(undefined));
    const select = container.querySelector<HTMLSelectElement>(".docs-language-menu select")!;
    for (const value of ["en", "same"]) {
      await act(async () => {
        select.value = value;
        select.dispatchEvent(new Event("change", { bubbles: true }));
      });
      await settle();
    }
    expect(saved).toEqual([{ docs_language: "en" }, { docs_language: null }]);
    // The page shows again in the new language
    expect(calls().filter((c) => c === "get_function").length).toBeGreaterThan(1);
  });
});

describe("the docs panel's notes", () => {
  it("shows a translated summary above documentation in another language, and says so", async () => {
    const { container } = await open(
      integrate({ summary: "es", body: "en", stale: false, summary_text: "Calcula simbólicamente la integral." }),
    );
    const summary = container.querySelector(".docs-translated-summary");
    expect(summary?.textContent).toBe("Calcula simbólicamente la integral.");
    expect(summary?.getAttribute("lang")).toBe("es");
    expect(status(container, "docs-body-language")?.textContent).toContain("The full documentation below is in English.");
  });

  it("says when a function isn't documented in the help's language", async () => {
    const { container } = await open(integrate({ summary: "en", body: "en", stale: false, fallback_from: "es" }));
    expect(status(container, "docs-fallback")?.textContent).toContain("isn't documented in Spanish");
  });

  it("warns that a translation may be out of date, with the English signatures", async () => {
    const { container } = await open(
      integrate({
        summary: "es",
        body: "es",
        stale: true,
        english_signatures: ["integrate(expr, x)", "integrate(expr, x, a, b)"],
      }),
    );
    const stale = status(container, "docs-stale");
    expect(stale?.textContent).toContain("may be out of date");
    expect([...stale!.querySelectorAll("code")].map((c) => c.textContent)).toEqual([
      "integrate(expr, x)",
      "integrate(expr, x, a, b)",
    ]);
  });

  it("names the pack that adds full documentation, with a copy button", async () => {
    const writeText = vi.fn(async () => {});
    vi.spyOn(navigator, "clipboard", "get").mockReturnValue({ writeText } as unknown as Clipboard);
    const info: DocsLanguages = {
      ...SPANISH_SUMMARIES,
      languages: [SPANISH_SUMMARIES.languages[0], { tag: "es", full: false, pack: "maxima-core-docs-es" }],
    };
    const { container } = await open(integrate(undefined), info);
    const strip = status(container, "docs-missing-pack");
    expect(strip?.querySelector("code")?.textContent).toBe("mxpm install maxima-core-docs-es");
    await click([...strip!.querySelectorAll("button")].find((b) => b.textContent === "Copy") ?? null);
    expect(writeText).toHaveBeenCalledWith("mxpm install maxima-core-docs-es");
  });

  it("says when no pack exists yet", async () => {
    const { container } = await open(integrate(undefined));
    expect(status(container, "docs-missing-pack")?.textContent).toContain("isn't available yet");
    expect(status(container, "docs-missing-pack")?.querySelector("code")).toBeNull();
  });

  it("shows nothing extra in English", async () => {
    const { container } = await open(integrate({ summary: "en", body: "en", stale: false }), {
      choice: null,
      effective: "en",
      languages: [{ tag: "en", full: true, pack: null }],
    });
    expect(container.querySelector(".docs-language-notes")?.children).toHaveLength(0);
  });
});
