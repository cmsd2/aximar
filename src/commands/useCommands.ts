// React's side of the command list: running a command with the session
// functions useMaxima gives, and the state that enables and checks them
import { useCallback, useEffect, useState } from "react";
import { useMaxima } from "../hooks/useMaxima";
import { useNotebookStore } from "../store/notebookStore";
import { useUiStore } from "../store/uiStore";
import { useLogStore } from "../store/logStore";
import { command, isEnabled, readCommandState, type CommandState } from "./registry";

/** A function that runs a command by id, if it is enabled; `cellId` for the cell context menu */
export function useRunCommand(): (id: string, cellId?: string) => Promise<void> {
  const { executeCell, runAll, restartSession } = useMaxima();
  return useCallback(
    async (id: string, cellId?: string) => {
      const c = command(id);
      if (!c) return;
      // A context-menu command is enabled for its own cell
      const state = readCommandState();
      if (!isEnabled(c, cellId ? stateFor(state, cellId) : state)) return;
      await c.run({ cellId, executeCell, runAll, restartSession });
    },
    [executeCell, runAll, restartSession],
  );
}

/** The state as if `cellId` were the active cell */
export function stateFor(state: CommandState, cellId: string): CommandState {
  const nb = useNotebookStore.getState();
  const tab = nb.activeNotebookId ? nb.notebooks[nb.activeNotebookId] : undefined;
  if (tab?.type !== "notebook") return state;
  const i = tab.cells.findIndex((c) => c.id === cellId);
  if (i === -1) return state;
  return {
    ...state,
    activeCellId: cellId,
    activeCellType: tab.cells[i].cellType,
    activeIsFirst: i === 0,
    activeIsLast: i === tab.cells.length - 1,
  };
}

function same(a: CommandState, b: CommandState): boolean {
  return (Object.keys(a) as (keyof CommandState)[]).every((k) => a[k] === b[k]);
}

/** The command state, kept up to date with the stores and with focus */
export function useCommandState(): CommandState {
  const [state, setState] = useState(readCommandState);
  useEffect(() => {
    const update = () =>
      setState((prev) => {
        const next = readCommandState();
        return same(prev, next) ? prev : next;
      });
    const unsubscribe = [useNotebookStore.subscribe(update), useUiStore.subscribe(update), useLogStore.subscribe(update)];
    document.addEventListener("focusin", update);
    document.addEventListener("focusout", update);
    update();
    return () => {
      for (const u of unsubscribe) u();
      document.removeEventListener("focusin", update);
      document.removeEventListener("focusout", update);
    };
  }, []);
  return state;
}
