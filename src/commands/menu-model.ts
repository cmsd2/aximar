// The menu bar as the window sends it to the backend (src-tauri/src/menu.rs),
// built from the command list in the spec's order
import type { I18n, MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { COMMANDS, isEnabled, type Command, type CommandState, type MenuName } from "./registry";
import { toAccelerator, type Platform } from "./shortcut";

export type Predefined =
  | "separator" | "cut" | "copy" | "paste" | "select_all" | "services" | "hide" | "hide_others"
  | "show_all" | "quit" | "about" | "minimize" | "maximize";

export type MenuNode =
  | { kind: "submenu"; label: string; items: MenuNode[] }
  | { kind: "item"; id: string; label: string; accelerator: string | null; enabled: boolean; checked: boolean | null }
  | { kind: "predefined"; which: Predefined; label: string | null };

/** One item's state, for set_menu_state */
export interface ItemState {
  id: string;
  enabled: boolean;
  checked: boolean | null;
}

const TITLES: Record<MenuName, MessageDescriptor> = {
  app: msg({ message: "Aximar", comment: "The application menu's title on macOS: the app's name, usually left as it is. max-length:12" }),
  file: msg({ message: "File", comment: "Menu bar: the File menu. max-length:12" }),
  edit: msg({ message: "Edit", comment: "Menu bar: the Edit menu. max-length:12" }),
  cell: msg({ message: "Cell", comment: "Menu bar: the menu of commands for notebook cells. max-length:12" }),
  session: msg({ message: "Session", comment: "Menu bar: the menu that interrupts and restarts Maxima. max-length:12" }),
  view: msg({ message: "View", comment: "Menu bar: the menu that shows and hides the panels. max-length:12" }),
  window: msg({ message: "Window", comment: "Menu bar: the Window menu. max-length:12" }),
  help: msg({ message: "Help", comment: "Menu bar: the Help menu. max-length:12" }),
};

const SYSTEM: Partial<Record<Predefined, MessageDescriptor>> = {
  about: msg({ message: "About Aximar", comment: "Application menu (macOS): shows Aximar's version and licence. max-length:30" }),
  services: msg({ message: "Services", comment: "Application menu (macOS): the system's Services submenu. max-length:30" }),
  hide: msg({ message: "Hide Aximar", comment: "Application menu (macOS): hides Aximar's windows. max-length:30" }),
  hide_others: msg({ message: "Hide Others", comment: "Application menu (macOS): hides the other apps' windows. max-length:30" }),
  show_all: msg({ message: "Show All", comment: "Application menu (macOS): shows every app's windows. max-length:30" }),
  quit: msg({ message: "Quit Aximar", comment: "Application menu (macOS): quits Aximar. max-length:30" }),
  cut: msg({ message: "Cut", comment: "Edit menu item: cuts the selected text. max-length:24" }),
  copy: msg({ message: "Copy", comment: "Edit menu item: copies the selected text. max-length:24" }),
  paste: msg({ message: "Paste", comment: "Edit menu item: pastes text. max-length:24" }),
  select_all: msg({ message: "Select All", comment: "Edit menu item: selects all the text in the field. max-length:24" }),
  minimize: msg({ message: "Minimize", comment: "Window menu item: minimises the window to the Dock or taskbar. max-length:30" }),
  maximize: msg({ message: "Zoom", comment: "Window menu item: makes the window fill the screen, or back. max-length:30" }),
};

const ORDER: MenuName[] = ["app", "file", "edit", "cell", "session", "view", "window", "help"];

/** Where the system's items go: before a menu's commands, after them, or after a group */
const SYSTEM_ITEMS: Partial<Record<MenuName, { before?: Predefined[]; afterGroup?: Record<number, Predefined[]>; after?: Predefined[] }>> = {
  app: {
    before: ["about"],
    after: ["separator", "services", "separator", "hide", "hide_others", "show_all", "separator", "quit"],
  },
  edit: { afterGroup: { 1: ["separator", "cut", "copy", "paste", "select_all"] } },
  window: { before: ["minimize", "maximize"] },
};

/** The commands in a menu, in list order, on this platform */
function commandsIn(name: MenuName, platform: Platform): Command[] {
  const own = COMMANDS.filter((c) => c.menu?.name === name && c.id !== "app.settings");
  const settings = COMMANDS.find((c) => c.id === "app.settings")!;
  if (platform === "macos" && name === "app") return [settings, ...own];
  // Elsewhere Settings ends the Edit menu
  if (platform !== "macos" && name === "edit") return [...own, settings];
  return own;
}

function itemNode(c: Command, i18n: I18n, state: CommandState): MenuNode {
  return {
    kind: "item",
    id: c.id,
    label: i18n._(c.label),
    accelerator: c.shortcut ? toAccelerator(c.shortcut) : null,
    enabled: isEnabled(c, state),
    checked: c.checked ? c.checked(state) : null,
  };
}

function system(which: Predefined, i18n: I18n): MenuNode {
  const label = SYSTEM[which];
  return { kind: "predefined", which, label: label ? i18n._(label) : null };
}

/** The menu bar in the window's language, with each item's state now */
export function menuModel(i18n: I18n, state: CommandState, platform: Platform): MenuNode[] {
  const bar: MenuNode[] = [];
  for (const name of ORDER) {
    // The application menu is macOS's; elsewhere Settings is in Edit
    if (name === "app" && platform !== "macos") continue;
    const sys = SYSTEM_ITEMS[name] ?? {};
    const items: MenuNode[] = (sys.before ?? []).map((w) => system(w, i18n));
    let group: number | undefined;
    for (const c of commandsIn(name, platform)) {
      const g = c.id === "app.settings" && name === "edit" ? 99 : (c.menu?.group ?? 0);
      if (group !== undefined && g !== group) {
        for (const w of sys.afterGroup?.[group] ?? []) items.push(system(w, i18n));
        items.push(system("separator", i18n));
      } else if (group === undefined && items.length > 0) {
        items.push(system("separator", i18n));
      }
      group = g;
      items.push(itemNode(c, i18n, state));
    }
    if (group !== undefined) for (const w of sys.afterGroup?.[group] ?? []) items.push(system(w, i18n));
    items.push(...(sys.after ?? []).map((w) => system(w, i18n)));
    bar.push({ kind: "submenu", label: i18n._(TITLES[name]), items });
  }
  return bar;
}

/** Every menu command's state, for set_menu_state */
export function menuState(state: CommandState): ItemState[] {
  return COMMANDS.filter((c) => c.menu || c.id === "app.settings").map((c) => ({
    id: c.id,
    enabled: isEnabled(c, state),
    checked: c.checked ? c.checked(state) : null,
  }));
}

/** The ids of every item in a model, for tests */
export function itemIds(nodes: MenuNode[]): string[] {
  return nodes.flatMap((n) => (n.kind === "submenu" ? itemIds(n.items) : n.kind === "item" ? [n.id] : []));
}
