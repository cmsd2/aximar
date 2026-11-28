import { useEffect, useRef, useState } from "react";
import { useLingui } from "@lingui/react/macro";
import type { I18n } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { command } from "../commands/registry";
import { menuModel, type MenuNode, type Predefined } from "../commands/menu-model";
import { formatShortcut } from "../commands/shortcut";
import { useCommandState, useRunCommand } from "../commands/useCommands";
import { IconCheck } from "./ui/icons";

type Submenu = Extract<MenuNode, { kind: "submenu" }>;

/** A row of an open menu */
export type Entry =
  | { kind: "separator"; key: string }
  | { kind: "action"; key: string; label: string; shortcut?: string; enabled: boolean; checked: boolean | null; run: () => void };

type Action = Extract<Entry, { kind: "action" }>;

const MAXIMIZE = msg({ message: "Maximize", comment: "Window menu (Windows): makes the window fill the screen, or back. max-length:30" });

/** The system's items' keys, which the page and the editor handle themselves */
const SYSTEM_KEYS: Partial<Record<Predefined, string>> = {
  cut: "Mod+X",
  copy: "Mod+C",
  paste: "Mod+V",
  select_all: "Mod+A",
};

/**
 * Paste the clipboard's text where the keyboard is.  The cell editor takes
 * a paste event; a plain text field needs the text typed.  The window
 * allows the page to read the clipboard (src-tauri/src/commands/window.rs).
 */
async function paste(): Promise<void> {
  let text: string;
  try {
    text = await navigator.clipboard.readText();
  } catch {
    return;
  }
  const target = document.activeElement ?? document.body;
  const data = new DataTransfer();
  data.setData("text/plain", text);
  const event = new ClipboardEvent("paste", { clipboardData: data, bubbles: true, cancelable: true });
  if (target.dispatchEvent(event)) document.execCommand("insertText", false, text);
}

function systemAction(which: Predefined): (() => void) | null {
  switch (which) {
    case "cut":
    case "copy":
      return () => document.execCommand(which);
    case "select_all":
      return () => document.execCommand("selectAll");
    case "paste":
      return () => void paste();
    case "minimize":
      return () => void getCurrentWindow().minimize();
    case "maximize":
      return () => void getCurrentWindow().toggleMaximize();
    default:
      // The rest are macOS's application menu, which Windows has none of
      return null;
  }
}

/** A menu's rows, with separators only between groups */
export function menuEntries(menu: Submenu, i18n: I18n, run: (id: string) => void): Entry[] {
  const out: Entry[] = [];
  menu.items.forEach((node, i) => {
    if (node.kind === "item") {
      const shortcut = command(node.id)?.shortcut;
      out.push({
        kind: "action",
        key: node.id,
        label: node.label,
        shortcut: shortcut ? formatShortcut(shortcut, "windows") : undefined,
        enabled: node.enabled,
        checked: node.checked,
        run: () => run(node.id),
      });
    } else if (node.kind === "predefined") {
      if (node.which === "separator") {
        if (out.length > 0 && out[out.length - 1].kind !== "separator") out.push({ kind: "separator", key: `sep-${i}` });
        return;
      }
      const action = systemAction(node.which);
      if (!action) return;
      // macOS calls it Zoom
      const label = node.which === "maximize" ? i18n._(MAXIMIZE) : (node.label ?? node.which);
      const key = SYSTEM_KEYS[node.which];
      out.push({
        kind: "action",
        key: node.which,
        label,
        shortcut: key ? formatShortcut(key, "windows") : undefined,
        enabled: true,
        checked: null,
        run: action,
      });
    }
  });
  while (out.length > 0 && out[out.length - 1].kind === "separator") out.pop();
  return out;
}

/** The next enabled row from `from`, `step` at a time, wrapping */
function nextEnabled(entries: Entry[], from: number, step: 1 | -1): number {
  const n = entries.length;
  for (let k = 1; k <= n; k++) {
    const i = (((from + step * k) % n) + n) % n;
    const e = entries[i];
    if (e.kind === "action" && e.enabled) return i;
  }
  return -1;
}

/**
 * The menu bar the page draws on Windows (lib/window-frame.ts), from the
 * same model as the native one elsewhere.  It never takes the keyboard's
 * focus, as a native menu doesn't: Undo, Cut and Copy act where the
 * keyboard was.  Alt alone highlights the first menu, Alt and a menu's
 * first letter opens it; the arrows, Enter and Escape work in it.
 */
export function MenuBar() {
  const { i18n } = useLingui();
  const state = useCommandState();
  const runCommand = useRunCommand();
  const menus = menuModel(i18n, state, "windows").filter((n): n is Submenu => n.kind === "submenu");
  const lists = menus.map((m) => menuEntries(m, i18n, (id) => void runCommand(id)));

  /** The open menu */
  const [open, setOpen] = useState<number | null>(null);
  /** The highlighted title, after Alt, with no menu open */
  const [hot, setHot] = useState<number | null>(null);
  /** The highlighted row of the open menu */
  const [active, setActive] = useState(-1);
  /** Opened from the keyboard: the menus' letters are underlined */
  const [keyboard, setKeyboard] = useState(false);
  const barRef = useRef<HTMLDivElement>(null);

  // The keys and mouse act on the latest state
  const live = useRef({ open, hot, active, keyboard, lists, menus });
  live.current = { open, hot, active, keyboard, lists, menus };

  const close = () => {
    setOpen(null);
    setHot(null);
    setActive(-1);
    setKeyboard(false);
  };
  const openMenu = (i: number, fromKeys: boolean) => {
    setOpen(i);
    setHot(null);
    setActive(fromKeys ? nextEnabled(live.current.lists[i], -1, 1) : -1);
    setKeyboard(fromKeys);
  };
  const choose = (entry: Action) => {
    if (!entry.enabled) return;
    close();
    entry.run();
  };

  useEffect(() => {
    let altAlone = false;
    const n = () => live.current.menus.length;
    const onKeyDown = (e: KeyboardEvent) => {
      const { open, hot, active, lists, menus } = live.current;
      if (e.key === "Alt") {
        altAlone = !e.ctrlKey && !e.shiftKey && !e.metaKey;
        return;
      }
      altAlone = false;
      const handled = () => {
        e.preventDefault();
        e.stopPropagation();
      };
      const plain = !e.ctrlKey && !e.metaKey && !e.shiftKey;
      // Alt and a menu's first letter, or the letter alone after Alt
      if (plain && e.key.length === 1 && (e.altKey || (hot !== null && open === null))) {
        const i = menus.findIndex((m) => m.label[0]?.toLowerCase() === e.key.toLowerCase());
        if (i !== -1) {
          handled();
          openMenu(i, true);
          return;
        }
      }
      if (open === null && hot === null) return;
      const at = open ?? hot!;
      switch (e.key) {
        case "ArrowLeft":
        case "ArrowRight": {
          const next = (at + (e.key === "ArrowRight" ? 1 : -1) + n()) % n();
          if (open !== null) openMenu(next, true);
          else setHot(next);
          return handled();
        }
        case "ArrowDown":
        case "ArrowUp":
          if (open === null) openMenu(at, true);
          else setActive(nextEnabled(lists[open], active, e.key === "ArrowDown" ? 1 : -1));
          setKeyboard(true);
          return handled();
        case "Home":
        case "End":
          if (open === null) return;
          setActive(e.key === "Home" ? nextEnabled(lists[open], -1, 1) : nextEnabled(lists[open], 0, -1));
          return handled();
        case "Enter":
        case " ": {
          if (open === null) {
            openMenu(at, true);
            return handled();
          }
          const entry = lists[open][active];
          if (entry?.kind === "action") {
            handled();
            choose(entry);
          }
          return;
        }
        case "Escape":
          close();
          return handled();
        default:
          // Any other key closes the menus and does its own work
          close();
      }
    };
    const onKeyUp = (e: KeyboardEvent) => {
      if (e.key !== "Alt" || !altAlone) return;
      altAlone = false;
      e.preventDefault();
      const { open, hot } = live.current;
      if (open !== null || hot !== null) close();
      else {
        setHot(0);
        setKeyboard(true);
      }
    };
    const onPointerDown = (e: MouseEvent) => {
      if (!barRef.current?.contains(e.target as Node)) close();
    };
    const onBlur = () => {
      altAlone = false;
      close();
    };
    window.addEventListener("keydown", onKeyDown, true);
    window.addEventListener("keyup", onKeyUp, true);
    document.addEventListener("mousedown", onPointerDown);
    window.addEventListener("blur", onBlur);
    return () => {
      window.removeEventListener("keydown", onKeyDown, true);
      window.removeEventListener("keyup", onKeyUp, true);
      document.removeEventListener("mousedown", onPointerDown);
      window.removeEventListener("blur", onBlur);
    };
  }, []);

  // Neither the titles nor the rows take focus
  const keepFocus = (e: React.MouseEvent) => e.preventDefault();

  return (
    <div ref={barRef} className={`menu-bar${keyboard ? " is-keyboard" : ""}`} role="menubar">
      {menus.map((menu, i) => (
        <div key={menu.label} className="menu-bar-item">
          <button
            type="button"
            tabIndex={-1}
            role="menuitem"
            aria-haspopup="menu"
            aria-expanded={open === i}
            className={`menu-title${open === i || hot === i ? " is-open" : ""}`}
            onMouseDown={keepFocus}
            onClick={() => (open === i ? close() : openMenu(i, false))}
            onMouseEnter={() => {
              if (open !== null && open !== i) openMenu(i, false);
            }}
          >
            <span className="menu-mnemonic">{menu.label.slice(0, 1)}</span>
            {menu.label.slice(1)}
          </button>
          {open === i && (
            <div className="menu-dropdown" role="menu" onMouseDown={keepFocus} onMouseLeave={() => setActive(-1)}>
              {lists[i].map((entry, j) =>
                entry.kind === "separator" ? (
                  <div key={entry.key} className="menu-separator" role="separator" />
                ) : (
                  <button
                    key={entry.key}
                    type="button"
                    tabIndex={-1}
                    role={entry.checked === null ? "menuitem" : "menuitemcheckbox"}
                    aria-checked={entry.checked === null ? undefined : entry.checked}
                    disabled={!entry.enabled}
                    className={`menu-item${active === j ? " is-active" : ""}`}
                    onMouseEnter={() => setActive(entry.enabled ? j : -1)}
                    onClick={() => choose(entry)}
                  >
                    <span className="menu-item-check">{entry.checked && <IconCheck size={16} stroke={1.75} aria-hidden="true" />}</span>
                    <span className="menu-item-label">{entry.label}</span>
                    {entry.shortcut && <kbd className="menu-item-key">{entry.shortcut}</kbd>}
                  </button>
                ),
              )}
            </div>
          )}
        </div>
      ))}
    </div>
  );
}
