// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { act } from "react";

const flags = vi.hoisted(() => ({ tools: true }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), ask: vi.fn(), message: vi.fn() }));
vi.mock("../../lib/translation-preview", () => ({
  get TRANSLATOR_TOOLS() {
    return flags.tools;
  },
  currentPreviewPath: () => null,
  startPreview: vi.fn(),
  stopPreview: vi.fn(),
}));

import { SettingsModal } from "../SettingsModal";
import type { SectionId } from "./sections";
import { answer, baseConfig, button, click, clientsStatus, mount, settle, unmount } from "./settings-test-kit";

const noop = () => {};

/** Each section's settings, by label */
const LABELS: Record<SectionId, string[]> = {
  general: ["Language", "Theme", "Variable panel open by default"],
  notebook: ["Font size", "Cell style", "Argument help", "Markdown font", "Markdown indent"],
  maxima: ["Backend", "Maxima path", "Evaluation timeout"],
  printing: ["Print font size", "Print margins (mm)"],
  "ai-assistants": ["MCP server", "Claude Code", "Codex", "Gemini CLI", "MCP listen address", "MCP token"],
  translation: ["Translation preview", "Show sources"],
};

/** Every setting the flat window had, by label, with Local's backend fields */
const OLD_WINDOW = [
  "Language", "Translation preview", "Show sources", "Theme", "Cell style", "Markdown font", "Markdown indent",
  "Argument help", "Backend", "Maxima path", "Font size", "Print font size", "Print margins (mm)", "Evaluation timeout",
  "MCP server", "MCP listen address", "MCP token", "Claude Code", "Codex", "Gemini CLI", "Variable panel open by default",
];

function labels(container: HTMLElement): string[] {
  return [...container.querySelectorAll(".settings-page .ui-field-label")].map((l) => l.textContent?.trim() ?? "");
}

function tabs(container: HTMLElement): string[] {
  return [...container.querySelectorAll('[role="tab"]')].map((t) => t.textContent?.trim() ?? "");
}

async function open(config: Record<string, unknown> = {}, initialSection?: SectionId) {
  const saved: Record<string, unknown>[] = [];
  answer({
    get_config: { config: { ...baseConfig, ...config }, warnings: [] },
    mcp_clients_status: clientsStatus,
    set_config: ({ updates }: Record<string, unknown>) => {
      saved.push(updates as Record<string, unknown>);
      return null;
    },
  });
  const onClose = vi.fn();
  const container = await mount(
    <SettingsModal onClose={onClose} onSetVariablesOpen={noop} initialSection={initialSection} />,
  );
  return { container, saved, onClose };
}

async function key(el: Element, k: string) {
  await act(async () => {
    el.dispatchEvent(new KeyboardEvent("keydown", { key: k, bubbles: true }));
  });
  await settle();
}

afterEach(() => {
  unmount();
  flags.tools = true;
});

describe("the Settings sections", () => {
  it.each(Object.entries(LABELS))("%s shows its settings and no others", async (id, expected) => {
    const { container } = await open({}, id as SectionId);
    if (id === "ai-assistants") container.querySelector("details")?.setAttribute("open", "");
    expect(labels(container)).toEqual(expected);
  });

  it("lose no setting from the flat window", async () => {
    const shown: string[] = [];
    for (const id of Object.keys(LABELS) as SectionId[]) {
      const { container } = await open({}, id);
      shown.push(...labels(container));
      unmount();
    }
    expect([...shown].sort()).toEqual([...OLD_WINDOW].sort());
  });

  it("give every setting a hint", async () => {
    for (const id of Object.keys(LABELS) as SectionId[]) {
      const { container } = await open({}, id);
      for (const row of container.querySelectorAll(".settings-page .ui-field-row")) {
        expect(row.querySelector(".ui-field-hint")?.textContent?.trim(), row.textContent ?? "").toBeTruthy();
      }
      unmount();
    }
  });

  it("include Translation in pre-release builds only", async () => {
    flags.tools = true;
    let { container } = await open();
    expect(tabs(container)).toEqual(["General", "Notebook", "Maxima", "Printing", "AI assistants", "Translation"]);
    unmount();
    flags.tools = false;
    ({ container } = await open());
    expect(tabs(container)).toEqual(["General", "Notebook", "Maxima", "Printing", "AI assistants"]);
  });
});

describe("the Maxima section", () => {
  it("shows Docker's fields for Docker", async () => {
    const { container } = await open({ backend: "docker" }, "maxima");
    expect(labels(container)).toEqual(["Backend", "Container engine", "Docker image", "Evaluation timeout"]);
  });

  it("keeps the Maxima path when switching back to Local", async () => {
    const { container } = await open({ maxima_path: "/opt/maxima/bin/maxima" }, "maxima");
    await click(button(container, "Docker"));
    expect(labels(container)).not.toContain("Maxima path");
    await click(button(container, "Local"));
    const field = container.querySelector<HTMLInputElement>(".settings-page input[type=text]");
    expect(field?.value).toBe("/opt/maxima/bin/maxima");
  });
});

describe("text fields", () => {
  async function typeInto(input: HTMLInputElement, text: string) {
    await act(async () => {
      const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, "value")!.set!;
      setter.call(input, text);
      input.dispatchEvent(new Event("input", { bubbles: true }));
    });
  }

  it("don't save until they lose focus", async () => {
    const { container, saved } = await open({}, "maxima");
    const field = container.querySelector<HTMLInputElement>(".settings-page input[type=text]")!;
    await typeInto(field, "/usr/bin/maxima");
    expect(saved.filter((u) => "maxima_path" in u)).toEqual([]);
    await act(async () => {
      field.dispatchEvent(new FocusEvent("focusout", { bubbles: true }));
    });
    await settle();
    expect(saved.filter((u) => "maxima_path" in u)).toEqual([{ maxima_path: "/usr/bin/maxima" }]);
  });

  it("save on Enter", async () => {
    const { container, saved } = await open({}, "maxima");
    const field = container.querySelector<HTMLInputElement>(".settings-page input[type=text]")!;
    await typeInto(field, "/usr/bin/maxima");
    await key(field, "Enter");
    expect(saved.filter((u) => "maxima_path" in u)).toEqual([{ maxima_path: "/usr/bin/maxima" }]);
  });

  it("put the saved value back on Esc, without closing Settings", async () => {
    const { container, saved, onClose } = await open({ maxima_path: "/a" }, "maxima");
    const field = container.querySelector<HTMLInputElement>(".settings-page input[type=text]")!;
    await typeInto(field, "/b");
    await key(field, "Escape");
    expect(field.value).toBe("/a");
    expect(onClose).not.toHaveBeenCalled();
    expect(saved.filter((u) => "maxima_path" in u)).toEqual([]);
  });
});

describe("the sidebar", () => {
  it("moves through the sections with the arrow keys", async () => {
    const { container } = await open({}, "general");
    const general = container.querySelector('[role="tab"][aria-selected="true"]')!;
    await key(general, "ArrowDown");
    await key(document.activeElement!, "ArrowDown");
    const selected = container.querySelector('[role="tab"][aria-selected="true"]');
    expect(selected?.textContent).toBe("Maxima");
    expect(document.activeElement).toBe(selected);
    expect(container.querySelector(".settings-page")?.getAttribute("data-section")).toBe("maxima");
  });

  it("jumps to the ends with Home and End", async () => {
    flags.tools = false;
    const { container } = await open({}, "notebook");
    await key(container.querySelector('[role="tab"][aria-selected="true"]')!, "End");
    expect(container.querySelector('[role="tab"][aria-selected="true"]')?.textContent).toBe("AI assistants");
    await key(document.activeElement!, "Home");
    expect(container.querySelector('[role="tab"][aria-selected="true"]')?.textContent).toBe("General");
  });

  it("remembers the section chosen", async () => {
    const { container, saved } = await open({}, "general");
    await click(button(container, "Printing"));
    expect(saved).toContainEqual({ settings_section: "printing" });
  });

  it("opens at the remembered section", async () => {
    const { container } = await open({ settings_section: "printing" });
    expect(container.querySelector(".settings-page")?.getAttribute("data-section")).toBe("printing");
  });

  it("opens at General when the remembered section isn't in this build", async () => {
    flags.tools = false;
    const { container } = await open({ settings_section: "translation" });
    expect(container.querySelector(".settings-page")?.getAttribute("data-section")).toBe("general");
  });
});

describe("closing", () => {
  it("happens with the close button", async () => {
    const { container, onClose } = await open({}, "general");
    await click(container.querySelector('button[aria-label="Close"]'));
    expect(onClose).toHaveBeenCalledOnce();
  });
});

describe("Esc", () => {
  it("closes Settings", async () => {
    const { container, onClose } = await open({}, "general");
    await key(container.querySelector('[role="tab"]')!, "Escape");
    expect(onClose).toHaveBeenCalledOnce();
  });

  it("in a menu leaves Settings open", async () => {
    const { container, onClose } = await open({}, "general");
    await key(container.querySelector(".settings-page select")!, "Escape");
    expect(onClose).not.toHaveBeenCalled();
  });
});
