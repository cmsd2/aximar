import { describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ close: vi.fn() }) }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ message: vi.fn(), ask: vi.fn(), open: vi.fn(), save: vi.fn() }));

import { i18n } from "../lib/i18n";
import { COMMANDS, readCommandState } from "./registry";
import { itemIds, menuModel, type MenuNode } from "./menu-model";

const titles = (bar: MenuNode[]) => bar.map((n) => (n.kind === "submenu" ? n.label : "?"));
const menu = (bar: MenuNode[], title: string) => {
  const m = bar.find((n) => n.kind === "submenu" && n.label === title);
  if (!m || m.kind !== "submenu") throw new Error(`no ${title} menu`);
  return m.items;
};
const labels = (items: MenuNode[]) =>
  items.map((n) => (n.kind === "item" ? n.label : n.kind === "predefined" ? (n.which === "separator" ? "—" : `[${n.which}]`) : n.label));

describe("the menu model", () => {
  it("has the menus in the spec's order, the app menu on macOS", () => {
    expect(titles(menuModel(i18n, readCommandState(), "macos"))).toEqual(["Aximar", "File", "Edit", "Cell", "Session", "View", "Window", "Help"]);
    expect(titles(menuModel(i18n, readCommandState(), "windows"))).toEqual(["File", "Edit", "Cell", "Session", "View", "Window", "Help"]);
  });

  it("makes every item a command, and every menu command an item", () => {
    const ids = itemIds(menuModel(i18n, readCommandState(), "macos"));
    const known = new Set(COMMANDS.map((c) => c.id));
    for (const id of ids) expect(known.has(id), id).toBe(true);
    for (const c of COMMANDS) if (c.menu) expect(ids, c.id).toContain(c.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("lays out Edit with the system's clipboard items after Undo and Redo", () => {
    const edit = labels(menu(menuModel(i18n, readCommandState(), "macos"), "Edit"));
    expect(edit).toEqual(["Undo", "Redo", "—", "[cut]", "[copy]", "[paste]", "[select_all]", "—", "Find", "Find and Replace"]);
  });

  it("puts Settings in Edit on Windows, and in the app menu on macOS", () => {
    const win = labels(menu(menuModel(i18n, readCommandState(), "windows"), "Edit"));
    expect(win.slice(-2)).toEqual(["—", "Settings..."]);
    const mac = labels(menu(menuModel(i18n, readCommandState(), "macos"), "Aximar"));
    expect(mac).toEqual(["[about]", "—", "Settings...", "—", "[services]", "—", "[hide]", "[hide_others]", "[show_all]", "—", "[quit]"]);
  });

  it("gives File its shortcuts as accelerators, and Run none", () => {
    const bar = menuModel(i18n, readCommandState(), "macos");
    const file = menu(bar, "File").filter((n) => n.kind === "item");
    expect(file.find((n) => n.kind === "item" && n.id === "file.close_tab")).toMatchObject({ accelerator: "CmdOrCtrl+W" });
    expect(file.find((n) => n.kind === "item" && n.id === "file.close_window")).toMatchObject({ accelerator: "CmdOrCtrl+Shift+W" });
    const run = menu(bar, "Cell").find((n) => n.kind === "item" && n.id === "cell.run");
    expect(run).toMatchObject({ accelerator: null });
  });

  it("makes the panels check items", () => {
    const view = menu(menuModel(i18n, readCommandState(), "linux"), "View");
    expect(view.find((n) => n.kind === "item" && n.id === "view.docs")).toMatchObject({ checked: false });
    expect(view.find((n) => n.kind === "item" && n.id === "view.palette")).toMatchObject({ checked: null });
  });
});
