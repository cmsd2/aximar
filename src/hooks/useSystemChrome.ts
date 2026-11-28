import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useNotebookStore } from "../store/notebookStore";
import { currentPlatform } from "../commands/shortcut";
import {
  SYSTEM_CHROME_VARIABLES,
  systemChromeVariables,
  type SystemChrome,
} from "../lib/system-chrome";

const DARK_QUERY = "(prefers-color-scheme: dark)";

/**
 * Paint the tab bar and toolbar in the desktop's chrome colours, and the
 * interface in its font (src/lib/system-chrome.ts).  Only Linux sends them.
 */
export function useSystemChrome() {
  const theme = useNotebookStore((s) => s.theme);
  const [chrome, setChrome] = useState<SystemChrome | null>(null);
  const [focused, setFocused] = useState(() => typeof document === "undefined" || document.hasFocus());
  const [systemDark, setSystemDark] = useState(
    () => typeof window !== "undefined" && !!window.matchMedia?.(DARK_QUERY).matches,
  );

  // The desktop's colours now, and again whenever its theme changes
  useEffect(() => {
    if (currentPlatform() !== "linux") return;
    let live = true;
    Promise.resolve(invoke<SystemChrome | null>("get_system_chrome"))
      .then((c) => live && setChrome(c ?? null))
      .catch(() => {});
    const unlisten = listen<SystemChrome>("system-chrome-changed", (e) => setChrome(e.payload));
    return () => {
      live = false;
      Promise.resolve(unlisten).then((fn) => fn?.());
    };
  }, []);

  // The menu bar dims when another window is active; so do the tabs
  useEffect(() => {
    const onFocus = () => setFocused(true);
    const onBlur = () => setFocused(false);
    window.addEventListener("focus", onFocus);
    window.addEventListener("blur", onBlur);
    return () => {
      window.removeEventListener("focus", onFocus);
      window.removeEventListener("blur", onBlur);
    };
  }, []);

  // Auto follows the system's light or dark
  useEffect(() => {
    const query = window.matchMedia?.(DARK_QUERY);
    if (!query) return;
    const onChange = () => setSystemDark(query.matches);
    query.addEventListener("change", onChange);
    return () => query.removeEventListener("change", onChange);
  }, []);

  useEffect(() => {
    const style = document.documentElement.style;
    const vars = systemChromeVariables(chrome, { theme, systemDark, focused });
    for (const name of SYSTEM_CHROME_VARIABLES) {
      const value = vars[name];
      if (value) style.setProperty(name, value);
      else style.removeProperty(name);
    }
  }, [chrome, theme, systemDark, focused]);
}
