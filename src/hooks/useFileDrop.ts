// Files dragged onto the window: what's being dragged, while it's over
// the window, and opening the notebooks when they're dropped
import { useEffect, useState } from "react";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { message } from "@tauri-apps/plugin-dialog";
import { t } from "@lingui/core/macro";
import { openDropped, sortDropped } from "../lib/file-drop";
import { openNotebookTab } from "./useNotebookEvents";

/** What a drag over the window holds: how many notebooks, how many other files */
export interface DragState {
  notebooks: number;
  others: number;
}

/** The drag over the window, or null; dropping opens the notebooks in it */
export function useFileDrop(): DragState | null {
  const [drag, setDrag] = useState<DragState | null>(null);

  useEffect(() => {
    let stop: (() => void) | undefined;
    let cancelled = false;
    getCurrentWebview()
      .onDragDropEvent(async ({ payload }) => {
        if (payload.type === "enter") {
          const { notebooks, others } = sortDropped(payload.paths);
          setDrag({ notebooks: notebooks.length, others: others.length });
        } else if (payload.type === "leave") {
          setDrag(null);
        } else if (payload.type === "drop") {
          setDrag(null);
          const failed = await openDropped(payload.paths, openNotebookTab);
          if (failed.length > 0) {
            await message(failed.map((f) => `${f.path}\n${f.reason}`).join("\n\n"), {
              title: t({
                message: "Couldn't Open Notebook",
                comment: "Title of the error dialog when a notebook file fails to open; the dialog shows the reason below",
              }),
              kind: "error",
            });
          }
        }
      })
      .then((unlisten) => {
        if (cancelled) unlisten();
        else stop = unlisten;
      });
    return () => {
      cancelled = true;
      stop?.();
    };
  }, []);

  return drag;
}
