// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { I18nProvider } from "@lingui/react";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const invoke = vi.fn(async (..._args: unknown[]) => undefined);
vi.mock("@tauri-apps/api/core", () => ({ invoke: (...a: unknown[]) => invoke(...a) }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
const win = { close: vi.fn(), minimize: vi.fn(), toggleMaximize: vi.fn() };
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => win }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ message: vi.fn(), ask: vi.fn(), open: vi.fn(), save: vi.fn() }));
vi.mock("../hooks/useMaxima", () => ({
  useMaxima: () => ({ executeCell: vi.fn(), runAll: vi.fn(), restartSession: vi.fn() }),
}));
const saveActive = vi.fn(async () => {});
vi.mock("../lib/file-commands", () => ({ saveActive: () => saveActive(), saveActiveAs: vi.fn(), openFromDisk: vi.fn() }));

import { i18n } from "../lib/i18n";
import { useNotebookStore } from "../store/notebookStore";
import { useUiStore } from "../store/uiStore";
import { setPlatformForTests } from "../commands/shortcut";
import { useMenuBar } from "../commands/useMenuBar";
import { MenuBar } from "./MenuBar";

function Host() {
  useMenuBar();
  return <MenuBar />;
}

describe("the menu bar the page draws on Windows", () => {
  let div: HTMLDivElement;
  let root: Root;

  beforeEach(async () => {
    setPlatformForTests("windows");
    invoke.mockClear();
    saveActive.mockClear();
    useUiStore.setState({ docsOpen: false });
    const nb = useNotebookStore.getState();
    nb.addTab("nb-1");
    nb.setActiveTab("nb-1");
    div = document.createElement("div");
    document.body.appendChild(div);
    root = createRoot(div);
    await act(async () =>
      root.render(
        <I18nProvider i18n={i18n}>
          <Host />
        </I18nProvider>,
      ),
    );
  });

  afterEach(async () => {
    await act(async () => root.unmount());
    div.remove();
    setPlatformForTests(null);
  });

  const titles = () => [...div.querySelectorAll<HTMLButtonElement>(".menu-title")];
  const title = (name: string) => titles().find((b) => b.textContent === name)!;
  const rows = () => [...div.querySelectorAll<HTMLButtonElement>(".menu-item")];
  const row = (name: string) => rows().find((b) => b.querySelector(".menu-item-label")?.textContent === name)!;
  const key = (k: string, init: KeyboardEventInit = {}, type = "keydown") =>
    act(async () => void window.dispatchEvent(new KeyboardEvent(type, { key: k, bubbles: true, cancelable: true, ...init })));

  it("sends no native menu", () => {
    expect(invoke.mock.calls.filter((c) => c[0] === "set_menu" || c[0] === "set_menu_state")).toEqual([]);
  });

  it("has Windows' menus, with Settings in Edit and no application menu", async () => {
    expect(titles().map((b) => b.textContent)).toEqual(["File", "Edit", "Cell", "Session", "View", "Window", "Help"]);
    await act(async () => title("Edit").click());
    expect(row("Settings...")).toBeDefined();
    expect(row("Copy").querySelector(".menu-item-key")?.textContent).toBe("Ctrl+C");
  });

  it("runs an item's command, showing its shortcut in Windows' names", async () => {
    await act(async () => title("File").click());
    expect(row("Save").querySelector(".menu-item-key")?.textContent).toBe("Ctrl+S");
    await act(async () => row("Save").click());
    expect(saveActive).toHaveBeenCalledTimes(1);
    expect(rows()).toEqual([]);
  });

  it("ticks a checked item", async () => {
    await act(async () => useUiStore.setState({ docsOpen: true }));
    await act(async () => title("View").click());
    expect(row("Docs").getAttribute("aria-checked")).toBe("true");
    expect(row("Variables").getAttribute("aria-checked")).toBe("false");
  });

  it("takes no focus, so commands act where the keyboard was", async () => {
    const field = document.createElement("textarea");
    div.appendChild(field);
    await act(async () => field.focus());
    const down = new MouseEvent("mousedown", { bubbles: true, cancelable: true });
    await act(async () => void title("Edit").dispatchEvent(down));
    expect(down.defaultPrevented).toBe(true);
    await act(async () => title("Edit").click());
    expect(document.activeElement).toBe(field);
  });

  it("opens a menu with Alt and its letter, and chooses with the arrows and Enter", async () => {
    await key("f", { altKey: true });
    expect(div.querySelector(".menu-bar")?.classList.contains("is-keyboard")).toBe(true);
    const first = rows()[0];
    expect(first.classList.contains("is-active")).toBe(true);
    // Down to Save: New Notebook, New Window, New from Template, Open, Save
    for (let i = 0; i < 4; i++) await key("ArrowDown");
    expect(div.querySelector(".menu-item.is-active .menu-item-label")?.textContent).toBe("Save");
    await key("Enter");
    expect(saveActive).toHaveBeenCalledTimes(1);
    expect(rows()).toEqual([]);
  });

  it("moves between menus with Left and Right, and closes with Escape", async () => {
    await key("f", { altKey: true });
    await key("ArrowRight");
    expect(title("Edit").getAttribute("aria-expanded")).toBe("true");
    await key("ArrowLeft");
    await key("ArrowLeft");
    expect(title("Help").getAttribute("aria-expanded")).toBe("true");
    await key("Escape");
    expect(rows()).toEqual([]);
  });

  it("highlights File on Alt alone, and opens it with Down", async () => {
    await key("Alt", { altKey: true });
    await key("Alt", {}, "keyup");
    expect(title("File").classList.contains("is-open")).toBe(true);
    expect(rows()).toEqual([]);
    await key("ArrowDown");
    expect(title("File").getAttribute("aria-expanded")).toBe("true");
  });

  it("runs the window's own items", async () => {
    await act(async () => title("Window").click());
    await act(async () => row("Minimize").click());
    expect(win.minimize).toHaveBeenCalledTimes(1);
  });
});
