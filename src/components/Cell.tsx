import { useCallback, useState, useEffect } from "react";
import { useLingui } from "@lingui/react/macro";
import type { Cell as CellType } from "../types/notebook";
import { getActiveTabState, useActiveTab, useNotebookStore } from "../store/notebookStore";
import { useFindStore } from "../store/findStore";
import { useMaxima } from "../hooks/useMaxima";
import { useCodeMirrorEditor } from "../hooks/useCodeMirrorEditor";
import { CellOutput } from "./CellOutput";
import { CellSuggestions } from "./CellSuggestions";
import { nbDeleteCell, nbMoveCell } from "../lib/notebook-commands";
import { afterRun, goTo } from "../lib/cell-caret";
import { cancelEvaluation } from "../lib/maxima-client";

interface CellProps {
  cell: CellType;
  onViewDocs?: (name: string) => void;
  selectBracket?: React.ReactNode;
}

export function Cell({ cell, onViewDocs, selectBracket }: CellProps) {
  const { t } = useLingui();
  const cells = useActiveTab().cells;
  const cellCount = cells.length;
  const setActiveCellId = useNotebookStore((s) => s.setActiveCellId);
  const { executeCell } = useMaxima();

  const [outputCollapsed, setOutputCollapsed] = useState(false);
  const hasFindMatch = useFindStore((s) => s.matches.some((m) => m.cellId === cell.id));

  // The next cell, or after the last one the caret below it
  const focusNextCell = useCallback(() => {
    goTo(afterRun(getActiveTabState().cells, cell.id));
  }, [cell.id]);

  const onExecute = useCallback(() => {
    executeCell(cell.id, cell.input).then((success) => {
      if (success) requestAnimationFrame(focusNextCell);
    });
  }, [cell.id, cell.input, executeCell, focusNextCell]);

  const onExecuteStay = useCallback(() => {
    executeCell(cell.id, cell.input);
  }, [cell.id, cell.input, executeCell]);

  const onSetActive = useCallback(() => {
    setActiveCellId(cell.id);
  }, [cell.id, setActiveCellId]);

  const { containerRef, syncExternalInput } = useCodeMirrorEditor({
    cellId: cell.id,
    initialValue: cell.input,
    onExecute,
    onExecuteStay,
    onFocusNext: focusNextCell,
    onSetActive,
    onViewDocs,
  });

  // Sync external changes (undo/redo, find-replace) into CM
  useEffect(() => {
    syncExternalInput(cell.input);
  }, [cell.input, syncExternalInput]);

  const execNum = cell.output?.executionCount ?? null;

  return (
    <>
    <div className={`cell ${cell.status}${outputCollapsed ? " output-collapsed" : ""}${hasFindMatch ? " has-find-match" : ""}`}>
      <div className="cell-input-area">
        {cell.output && (
          <button
            className="bracket-toggle"
            onClick={() => setOutputCollapsed((c) => !c)}
            title={
              outputCollapsed
                ? t({
                    message: "Expand output",
                    comment: "Tooltip of the bracket beside a code cell: shows the cell's hidden output",
                  })
                : t({
                    message: "Collapse output",
                    comment: "Tooltip of the bracket beside a code cell: hides the cell's output",
                  })
            }
          />
        )}
        <div className="cell-gutter">
          {cell.status === "running" ? (
            <span className="cell-indicator running">*</span>
          ) : (
            <span className="cell-indicator">
              {execNum
                ? t({
                    message: `In [${execNum}]`,
                    comment:
                      "Label left of a code cell's input, as in Jupyter. execNum is how many cells the session had run when this one last ran. max-length:10",
                  })
                : t({
                    message: "In",
                    comment: "Label left of the input of a code cell that has not run yet, as in Jupyter. max-length:6",
                  })}
            </span>
          )}
        </div>
        <div
          ref={containerRef}
          className="cell-input"
          data-cell-id={cell.id}
        />
        <div className="cell-actions">
          {cellCount > 1 && (
            <>
              <button
                className="cell-btn move-btn"
                onClick={() => nbMoveCell(cell.id, "up")}
                title={t({ message: "Move cell up", comment: "Tooltip of a code cell's up-arrow button" })}
                disabled={cells[0]?.id === cell.id}
              >
                &#9650;
              </button>
              <button
                className="cell-btn move-btn"
                onClick={() => nbMoveCell(cell.id, "down")}
                title={t({ message: "Move cell down", comment: "Tooltip of a code cell's down-arrow button" })}
                disabled={cells[cells.length - 1]?.id === cell.id}
              >
                &#9660;
              </button>
            </>
          )}
          {cell.status === "running" ? (
            <button
              className="cell-btn stop-btn"
              onClick={() => cancelEvaluation()}
              title={t({
                message: "Cancel cell",
                comment: "Tooltip of the stop button on a running code cell: interrupts Maxima",
              })}
            >
              &#9632;
            </button>
          ) : (
            <button
              className="cell-btn run-btn"
              onClick={() => executeCell(cell.id, cell.input)}
              title={t({
                message: "Run cell (Shift+Enter)",
                comment: "Tooltip of a code cell's run button. Shift+Enter is the keyboard shortcut; keep it as it is",
              })}
            >
              &#9654;
            </button>
          )}
          {cellCount > 1 && (
            <button
              className="cell-btn delete-btn"
              onClick={() => nbDeleteCell(cell.id)}
              title={t({ message: "Delete cell", comment: "Tooltip of the cross button that deletes a cell" })}
            >
              &times;
            </button>
          )}
        </div>
      </div>
      {cell.output && !outputCollapsed && (
        <div
          className={`cell-output-area${cell.output.stale ? " stale" : ""}`}
          title={
            cell.output.stale
              ? t({
                  message: "From an earlier session. Run the cell to update it.",
                  comment: "Tooltip of a dimmed cell output that was saved in the file and not produced by this session",
                })
              : undefined
          }
        >
          <div className="cell-gutter">
            <span className="cell-indicator">
              {execNum
                ? t({
                    message: `Out [${execNum}]`,
                    comment:
                      "Label left of a code cell's output, as in Jupyter. execNum is how many cells the session had run when this one last ran. max-length:10",
                  })
                : t({
                    message: "Out",
                    comment: "Label left of the output of a code cell with no run number, as in Jupyter. max-length:6",
                  })}
            </span>
          </div>
          <CellOutput output={cell.output} cellId={cell.id} />
        </div>
      )}
      {selectBracket}
    </div>
    <CellSuggestions cell={cell} />
    </>
  );
}
