import { message } from "@tauri-apps/plugin-dialog";
import { plural, t } from "@lingui/core/macro";
import { useNotebookStore } from "../store/notebookStore";
import { displayTitle } from "./tab-title";
import { saveNotebook } from "./notebooks-client";

// The dialog reports the label of the button clicked, so choiceFor reads
// these back in the same language
function saveLabel(): string {
  return t({
    message: "Save",
    comment: "Button in the Unsaved Changes dialog: saves the notebooks, then closes them",
  });
}

function dontSaveLabel(): string {
  return t({
    message: "Don't Save",
    comment: "Button in the Unsaved Changes dialog: closes the notebooks and loses the changes",
  });
}

function cancelLabel(): string {
  return t({
    message: "Cancel",
    comment: "Button in the Unsaved Changes dialog: keeps the notebooks open",
  });
}

/** The question in the dialog, naming the one notebook or counting them */
function saveQuestion(names: string[]): string {
  if (names.length === 1) {
    const name = names[0];
    return t({
      message: `Do you want to save the changes you made to "${name}"?`,
      comment: "Unsaved Changes dialog, when closing one notebook with changes. name is its title, such as calc.macnb",
    });
  }
  const count = names.length;
  return t({
    message: plural(count, {
      one: "Do you want to save the changes you made to # notebook?",
      other: "Do you want to save the changes you made to # notebooks?",
    }),
    comment: "Unsaved Changes dialog, when closing several notebooks with changes. The number is how many",
  });
}

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
  const warning = t({
    message: "Your changes will be lost if you don't save them.",
    comment: "Unsaved Changes dialog: the line under the question",
  });
  const result = await message(`${saveQuestion(names)}\n\n${warning}`, {
    title: t({
      message: "Unsaved Changes",
      comment: "Title of the dialog asking whether to save notebooks before closing them",
    }),
    kind: "warning",
    buttons: { yes: saveLabel(), no: dontSaveLabel(), cancel: cancelLabel() },
  });
  return choiceFor(result);
}

/** The choice a dialog result stands for.  Anything unexpected keeps the
 *  notebook open. */
export function choiceFor(result: string): UnsavedChoice {
  if (result === saveLabel() || result === "Yes") return "save";
  if (result === dontSaveLabel() || result === "No") return "discard";
  return "cancel";
}

/**
 * Save a notebook tab to its file, asking for a file name if it has none.
 * Returns false when the user cancels the file dialog.
 */
export async function saveNotebookTab(notebookId: string): Promise<boolean> {
  const tab = useNotebookStore.getState().notebooks[notebookId];
  if (!tab || tab.type !== "notebook") return true;
  // The path_changed event sets the tab's title and save path
  const savedPath = await saveNotebook(notebookId, tab.filePath);
  if (!savedPath) return false;
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

  const choice = await askToSave(dirty.map((id) => displayTitle(notebooks[id].title)));
  if (choice === "cancel") return false;
  if (choice === "discard") return true;

  for (const id of dirty) {
    if (dirty.length > 1) useNotebookStore.getState().setActiveTab(id);
    if (!(await saveNotebookTab(id))) return false;
  }
  return true;
}
