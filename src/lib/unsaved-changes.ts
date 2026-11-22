import { message } from "@tauri-apps/plugin-dialog";
import { useNotebookStore } from "../store/notebookStore";
import { saveNotebook } from "./notebooks-client";
import { nbSetPath } from "./notebook-commands";

const SAVE = "Save";
const DONT_SAVE = "Don't Save";
const CANCEL = "Cancel";

export type UnsavedChoice = "save" | "discard" | "cancel";

/**
 * Ask what to do with unsaved changes before closing, as other apps do:
 * Save (the default), Don't Save, or Cancel.
 *
 * The order matters.  The dialog plugin makes the first button the default
 * (Return), and on Windows and Linux it reports Escape as the last button,
 * so Save is first and Cancel is last: neither key loses work.
 */
export async function askToSave(names: string[]): Promise<UnsavedChoice> {
  const what = names.length === 1 ? `"${names[0]}"` : `${names.length} notebooks`;
  const result = await message(
    `Do you want to save the changes you made to ${what}?\n\nYour changes will be lost if you don't save them.`,
    {
      title: "Unsaved Changes",
      kind: "warning",
      buttons: { yes: SAVE, no: DONT_SAVE, cancel: CANCEL },
    },
  );
  return choiceFor(result);
}

/** The choice a dialog result stands for.  Anything unexpected keeps the
 *  notebook open. */
export function choiceFor(result: string): UnsavedChoice {
  if (result === SAVE || result === "Yes") return "save";
  if (result === DONT_SAVE || result === "No") return "discard";
  return "cancel";
}

/**
 * Save a notebook tab to its file, asking for a file name if it has none.
 * Returns false when the user cancels the file dialog.
 */
export async function saveNotebookTab(notebookId: string): Promise<boolean> {
  const tab = useNotebookStore.getState().notebooks[notebookId];
  if (!tab || tab.type !== "notebook") return true;
  const savedPath = await saveNotebook(tab.cells, tab.filePath);
  if (!savedPath) return false;
  // The path_changed event sets the tab's title and save path
  await nbSetPath(notebookId, savedPath);
  useNotebookStore.getState().markNotebookClean(notebookId);
  return true;
}

/**
 * Whether these tabs may close: true when none has unsaved changes, or the
 * user saved or discarded them.  With several, one dialog covers them all,
 * and Save saves each in turn, showing its tab while its file dialog is open.
 */
export async function confirmClose(notebookIds: string[]): Promise<boolean> {
  const { notebooks } = useNotebookStore.getState();
  const dirty = notebookIds.filter((id) => notebooks[id]?.isDirty);
  if (dirty.length === 0) return true;

  const choice = await askToSave(dirty.map((id) => notebooks[id].title));
  if (choice === "cancel") return false;
  if (choice === "discard") return true;

  for (const id of dirty) {
    if (dirty.length > 1) useNotebookStore.getState().setActiveTab(id);
    if (!(await saveNotebookTab(id))) return false;
  }
  return true;
}
