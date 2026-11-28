// The window's side of the native menu bar: it sends the menu on load and
// when the language changes, the items' state when it has focus and that
// state changes, and runs the command a chosen item names.  On Windows the
// page draws the menu bar instead (components/MenuBar.tsx), and nothing is sent.
import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { i18n } from "../lib/i18n";
import { readCommandState, shortcutCommand } from "./registry";
import { menuModel, menuState, type ItemState } from "./menu-model";
import { currentPlatform } from "./shortcut";
import { drawsWindowFrame } from "../lib/window-frame";
import { useCommandState, useRunCommand } from "./useCommands";

/** Send the whole menu bar, in the window's language, unless the page draws it */
export async function sendMenu(): Promise<void> {
  if (drawsWindowFrame()) return;
  return invoke("set_menu", { model: menuModel(i18n, readCommandState(), currentPlatform()) });
}

async function sendState(states: ItemState[]): Promise<void> {
  if (drawsWindowFrame()) return;
  return invoke("set_menu_state", { states });
}

export function useMenuBar(): void {
  const run = useRunCommand();
  const state = useCommandState();
  const sent = useRef<string>("");

  // The menu on load and on a language change, including a translation preview
  useEffect(() => {
    void sendMenu();
    return i18n.on("change", () => {
      void sendMenu();
    });
  }, []);

  // The items' state, while this window has focus, when it changes
  useEffect(() => {
    if (!document.hasFocus()) return;
    const states = menuState(state);
    const key = JSON.stringify(states);
    if (key === sent.current) return;
    sent.current = key;
    void sendState(states);
  }, [state]);

  // Taking focus from another window: this window's state replaces its
  useEffect(() => {
    const onFocus = () => {
      const states = menuState(readCommandState());
      sent.current = JSON.stringify(states);
      void sendState(states);
    };
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  }, []);

  // A menu item, run in this window, the focused one
  const runRef = useRef(run);
  runRef.current = run;
  const lastKey = useRef<{ id: string; at: number } | null>(null);
  useEffect(() => {
    const unlisten = listen<{ id: string }>("menu-command", (event) => {
      const { id } = event.payload;
      // The menu's echo of a shortcut the page has already run
      const k = lastKey.current;
      if (k && k.id === id && Date.now() - k.at < ECHO_MS) return;
      void runRef.current(id);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // On Windows a key pressed in the page never reaches the menu's
  // accelerators: WebView2 takes the keyboard in its own process.  The
  // page runs the menu's shortcuts itself, before the cell editor sees
  // them, as the menu takes them first on macOS and Linux.
  useEffect(() => {
    if (currentPlatform() !== "windows") return;
    const onKey = (e: KeyboardEvent) => {
      const c = shortcutCommand(e);
      if (!c) return;
      e.preventDefault();
      e.stopPropagation();
      lastKey.current = { id: c.id, at: Date.now() };
      void runRef.current(c.id);
    };
    window.addEventListener("keydown", onKey, true);
    return () => window.removeEventListener("keydown", onKey, true);
  }, []);
}

/** How soon after a shortcut the menu's own event for it counts as the same press */
const ECHO_MS = 250;
