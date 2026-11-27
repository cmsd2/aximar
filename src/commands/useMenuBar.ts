// The window's side of the native menu bar: it sends the menu on load and
// when the language changes, the items' state when it has focus and that
// state changes, and runs the command a chosen item names
import { useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { i18n } from "../lib/i18n";
import { readCommandState } from "./registry";
import { menuModel, menuState, type ItemState } from "./menu-model";
import { currentPlatform } from "./shortcut";
import { useCommandState, useRunCommand } from "./useCommands";

/** Send the whole menu bar, in the window's language */
export function sendMenu(): Promise<void> {
  return invoke("set_menu", { model: menuModel(i18n, readCommandState(), currentPlatform()) });
}

function sendState(states: ItemState[]): Promise<void> {
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
  useEffect(() => {
    const unlisten = listen<{ id: string }>("menu-command", (event) => {
      void runRef.current(event.payload.id);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);
}
