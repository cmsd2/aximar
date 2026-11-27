// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
const listeners: Record<string, () => void> = {};
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (name: string, handler: () => void) => {
    listeners[name] = handler;
    return () => {};
  }),
}));

import { DocsPanel } from "../DocsPanel";
import { HelpLanguageRow } from "../settings/HelpLanguageRow";
import { useSettingsStore } from "../../store/settingsStore";
import type { AppConfig } from "../../lib/config-client";
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

describe("the help language", () => {
  async function row(docs_language: string | null) {
    const saved: unknown[] = [];
    answer({ docs_languages: SPANISH_SUMMARIES });
    const container = await mount(
      <HelpLanguageRow config={{ docs_language } as AppConfig} update={(u) => saved.push(u)} />,
    );
    await settle();
    return { container, saved, select: container.querySelector<HTMLSelectElement>("select")! };
  }

  it("is in Settings, offering Aximar's language and each language with documentation, by its own name", async () => {
    const { container, select } = await row(null);
    expect(container.textContent).toContain("Help language");
    expect([...select.options].map((o) => o.textContent)).toEqual(["Same as Aximar's language", "English", "Español"]);
    expect(select.value).toBe("same");
  });

  it("saves a choice, and Aximar's language as null", async () => {
    const { select, saved } = await row("es");
    expect(select.value).toBe("es");
    for (const value of ["en", "same"]) {
      await act(async () => {
        select.value = value;
        select.dispatchEvent(new Event("change", { bubbles: true }));
      });
    }
    expect(saved).toEqual([{ docs_language: "en" }, { docs_language: null }]);
  });

  it("isn't in the docs panel, which shows its page again when the setting changes", async () => {
    const { container } = await open(integrate(undefined));
    expect(container.querySelector(".docs-panel-header select")).toBeNull();
    const before = calls().filter((c) => c === "get_function").length;
    await act(async () => listeners["docs-language-changed"]());
    await settle();
    expect(calls().filter((c) => c === "get_function").length).toBeGreaterThan(before);
  });

  it("is a click away from a note about the help's language", async () => {
    const { container } = await open(
      integrate({ summary: "es", body: "en", stale: false, summary_text: "Calcula simbólicamente la integral." }),
    );
    const link = container.querySelector<HTMLButtonElement>(".docs-help-language-link")!;
    expect(link.textContent).toBe("Help language settings");
    await act(async () => link.click());
    expect(useSettingsStore.getState().open).toBe(true);
    expect(useSettingsStore.getState().section).toBe("general");
  });

  it("isn't linked when the help shows in its own language", async () => {
    const { container } = await open(integrate(undefined), { ...SPANISH_SUMMARIES, effective: "en" });
    expect(container.querySelector(".docs-help-language-link")).toBeNull();
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
