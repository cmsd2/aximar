// Closing, switching and moving tabs, for the tab bar, the File and Window
// menus and their shortcuts.  Moved from TabBar.tsx so commands can run them.
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useNotebookStore } from "../store/notebookStore";
import { confirmClose } from "./unsaved-changes";
import { createWindow, nbClose, nbSetActive } from "./notebook-commands";

function tabIds(): string[] {
  return Object.keys(useNotebookStore.getState().notebooks);
}

/**
 * Close a tab, asking about unsaved changes.  The last tab closes the
 * window instead, through the window's own close check.
 */
export async function closeTab(id: string): Promise<void> {
  if (tabIds().length <= 1) {
    await closeWindow();
    return;
  }
  const tab = useNotebookStore.getState().notebooks[id];
  // Save, Don't Save or Cancel when it has unsaved changes
  if (!(await confirmClose([id]))) return;
  try {
    // Only close the backend notebook for notebook tabs
    if (!tab || tab.type === "notebook") await nbClose(id);
    useNotebookStore.getState().removeTab(id);
  } catch (e) {
    console.warn("Failed to close tab:", e);
  }
}

/** Close the active tab */
export async function closeActiveTab(): Promise<void> {
  const id = useNotebookStore.getState().activeNotebookId;
  if (id) await closeTab(id);
}

/** Close the window; its close handler asks about unsaved changes */
export async function closeWindow(): Promise<void> {
  await getCurrentWindow().close();
}

/** Make a tab the active one */
export async function switchTab(id: string): Promise<void> {
  const store = useNotebookStore.getState();
  store.setActiveTab(id);
  // Only notify the backend for notebook tabs
  if (store.notebooks[id]?.type === "notebook") await nbSetActive(id);
}

/** Switch to the next tab, or the previous one, wrapping round */
export async function stepTab(dir: 1 | -1): Promise<void> {
  const ids = tabIds();
  const active = useNotebookStore.getState().activeNotebookId;
  if (ids.length < 2 || !active) return;
  const i = ids.indexOf(active);
  await switchTab(ids[(i + dir + ids.length) % ids.length]);
}

/** Move a tab to a window of its own; the last tab stays */
export async function moveTabToNewWindow(id: string): Promise<void> {
  if (tabIds().length <= 1) return;
  await createWindow(id);
  useNotebookStore.getState().removeTab(id);
}
