import { useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useNotebookStore, type Theme } from "../store/notebookStore";

export function useTheme() {
  const theme = useNotebookStore((s) => s.theme);
  const setTheme = useNotebookStore((s) => s.setTheme);

  // Load saved theme from backend on mount
  useEffect(() => {
    invoke<string>("get_theme").then((saved) => {
      if (saved === "light" || saved === "dark" || saved === "auto") {
        setTheme(saved as Theme);
      }
    });
  }, [setTheme]);

  // Apply theme to DOM and persist to backend
  useEffect(() => {
    if (theme === "auto") {
      delete document.documentElement.dataset.theme;
    } else {
      document.documentElement.dataset.theme = theme;
    }
    invoke("set_theme", { theme });
  }, [theme]);
}
