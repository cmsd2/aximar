// Open and save, for the File menu and its shortcuts.  Moved from App.tsx so
// commands can run them; what they do is unchanged.
import { message } from "@tauri-apps/plugin-dialog";
import { t } from "@lingui/core/macro";
import { useNotebookStore } from "../store/notebookStore";
import { saveNotebookTab } from "./unsaved-changes";
import { openFile, saveNotebookAs, savePlotFile } from "./notebooks-client";
import { openNotebookTab } from "../hooks/useNotebookEvents";

/** Save the active tab: a notebook to its file, or ask where; a plot as JSON */
export async function saveActive(): Promise<void> {
  const state = useNotebookStore.getState();
  const id = state.activeNotebookId;
  const tab = id ? state.notebooks[id] : undefined;
  if (!id || !tab) return;
  if (tab.type === "plot") {
    await savePlotFile(tab.plotData, tab.filePath);
  } else {
    await saveNotebookTab(id);
  }
}

/** Save the active tab under a new name */
export async function saveActiveAs(): Promise<void> {
  const state = useNotebookStore.getState();
  const id = state.activeNotebookId;
  const tab = id ? state.notebooks[id] : undefined;
  if (!id || !tab) return;
  if (tab.type === "plot") {
    await savePlotFile(tab.plotData, null);
  } else {
    // The path_changed event sets the tab's title and save path
    const savedPath = await saveNotebookAs(id, tab.filePath);
    if (savedPath) useNotebookStore.getState().markNotebookClean(id);
  }
}

/** Ask for a notebook or plot file and open it in a new tab */
export async function openFromDisk(): Promise<void> {
  const result = await openFile();
  if (!result) return;
  if (result.type === "plot") {
    // Open Plotly JSON as a plot viewer tab
    const title =
      result.path.split("/").pop()?.split("\\").pop() ??
      t({ message: "Plot", comment: "Tab title for an opened plot file whose file name could not be read" });
    useNotebookStore.getState().addPlotTab(`plot-${crypto.randomUUID()}`, title, result.plotData, result.path);
    return;
  }
  try {
    await openNotebookTab(result.path);
  } catch (e) {
    await message(String(e), {
      title: t({
        message: "Couldn't Open Notebook",
        comment: "Title of the error dialog when a notebook file fails to open; the dialog shows the reason below",
      }),
      kind: "error",
    });
  }
}
