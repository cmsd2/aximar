import { useCallback, useState, useEffect } from "react";
import type { Cell as CellType } from "../types/notebook";
import { useActiveTab, useNotebookStore } from "../store/notebookStore";
import { useFindStore } from "../store/findStore";
import { useMaxima } from "../hooks/useMaxima";
import { useCodeMirrorEditor } from "../hooks/useCodeMirrorEditor";
import { CellOutput } from "./CellOutput";
import { CellSuggestions } from "./CellSuggestions";
import { nbDeleteCell, nbMoveCell, nbAddCell, nbApproveCell, nbAbortCell } from "../lib/notebook-commands";

interface CellProps {
  cell: CellType;
  onViewDocs?: (name: string) => void;
  selectBracket?: React.ReactNode;
}

export function Cell({ cell, onViewDocs, selectBracket }: CellProps) {
  const cells = useActiveTab().cells;
  const cellCount = cells.length;
  const setActiveCellId = useNotebookStore((s) => s.setActiveCellId);
  const { executeCell } = useMaxima();

  const [outputCollapsed, setOutputCollapsed] = useState(false);
  const hasFindMatch = useFindStore((s) => s.matches.some((m) => m.cellId === cell.id));

  const focusNextCell = useCallback(() => {
    const allCells = Array.from(
      document.querySelectorAll<HTMLElement>(".cell")
    );
    const thisCell = containerRef.current?.closest(".cell");
    const idx = thisCell ? allCells.indexOf(thisCell as HTMLElement) : -1;
    if (idx !== -1 && idx + 1 < allCells.length) {
      const next = allCells[idx + 1];
      // Try CodeMirror editor first (code cell), fall back to double-clicking (markdown cell)
      const cmContent = next.querySelector<HTMLElement>(".cm-content");
      if (cmContent) {
        cmContent.focus();
      } else {
        const view = next.querySelector<HTMLElement>(".markdown-cell-view");
        view?.dispatchEvent(new MouseEvent("dblclick", { bubbles: true }));
      }
    }
  }, []);

  const onExecute = useCallback(() => {
    const idx = cells.findIndex((c) => c.id === cell.id);
    const isLastCell = idx === cells.length - 1;

    executeCell(cell.id, cell.input).then((success) => {
      if (!success) return;
      if (isLastCell) {
        nbAddCell("code", undefined, cell.id);
      }
      requestAnimationFrame(focusNextCell);
    });
  }, [cell.id, cell.input, cells, executeCell, focusNextCell]);

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
            title={outputCollapsed ? "Expand output" : "Collapse output"}
          />
        )}
        <div className="cell-gutter">
          {cell.status === "running" ? (
            <span className="cell-indicator running">*</span>
          ) : cell.status === "pending_approval" ? (
            <span className="cell-indicator pending-approval">!</span>
          ) : (
            <span className="cell-indicator">
              {execNum ? `In [${execNum}]` : "In"}
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
                title="Move cell up"
                disabled={cells[0]?.id === cell.id}
              >
                &#9650;
              </button>
              <button
                className="cell-btn move-btn"
                onClick={() => nbMoveCell(cell.id, "down")}
                title="Move cell down"
                disabled={cells[cells.length - 1]?.id === cell.id}
              >
                &#9660;
              </button>
            </>
          )}
          <button
            className="cell-btn run-btn"
            onClick={() => executeCell(cell.id, cell.input)}
            title="Run cell (Shift+Enter)"
          >
            &#9654;
          </button>
          {cellCount > 1 && (
            <button
              className="cell-btn delete-btn"
              onClick={() => nbDeleteCell(cell.id)}
              title="Delete cell"
            >
              &times;
            </button>
          )}
        </div>
      </div>
      {cell.status === "pending_approval" && (
        <div className="cell-approval-bar">
          <span className="approval-warning">
            &#9888; Dangerous: {cell.dangerousFunctions?.join(", ")}
          </span>
          <button className="cell-btn approve-btn" onClick={() => nbApproveCell(cell.id)}>
            Approve
          </button>
          <button className="cell-btn abort-btn" onClick={() => nbAbortCell(cell.id)}>
            Abort
          </button>
        </div>
      )}
      {cell.output && !outputCollapsed && (
        <div className="cell-output-area">
          <div className="cell-gutter">
            <span className="cell-indicator">
              {execNum ? `Out [${execNum}]` : "Out"}
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
