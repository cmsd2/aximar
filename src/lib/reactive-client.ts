import { invoke } from "@tauri-apps/api/core";
import { useNotebookStore } from "../store/notebookStore";
import type { EvalResult } from "../types/maxima";

function activeId(): string | null {
  return useNotebookStore.getState().activeNotebookId;
}

export interface ReactiveSignalMeta {
  name: string;
  lo: number;
  hi: number;
  value: number;
  kind: string;
}

export interface ReactiveBlock {
  view_id: string;
  signals: ReactiveSignalMeta[];
}

export async function setSignalAndReplot(
  viewId: string,
  name: string,
  value: number,
): Promise<EvalResult> {
  return invoke<EvalResult>("set_signal_and_replot", {
    notebookId: activeId(),
    viewId,
    name,
    value,
  });
}
