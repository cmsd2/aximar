import { useCallback, useState, useEffect } from "react";
import type { Cell as CellType } from "../types/notebook";
import { useNotebookStore } from "../store/notebookStore";
import { useFindStore } from "../store/findStore";
import { useMaxima } from "../hooks/useMaxima";
import { useCodeMirrorEditor } from "../hooks/useCodeMirrorEditor";
import { CellOutput } from "./CellOutput";
import { CellSuggestions } from "./CellSuggestions";
import { nbDeleteCell, nbMoveCell, nbAddCell } from "../lib/notebook-commands";

interface CellProps {
  cell: CellType;
  onViewDocs?: (name: string) => void;
}

export function Cell({ cell, onViewDocs }: CellProps) {
  const cells = useNotebookStore((s) => s.cells);
  const cellCount = cells.length;
  const setActiveCellId = useNotebookStore((s) => s.setActiveCellId);
  const { executeCell } = useMaxima();

  const [outputCollapsed, setOutputCollapsed] = useState(false);
  const hasFindMatch = useFindStore((s) => s.matches.some((m) => m.cellId === cell.id));

  const focusNextCell = useCallback(() => {
    const allInputs = Array.from(
      document.querySelectorAll<HTMLDivElement>(".cell-input")
    );
    const container = containerRef.current;
    const currentIdx = container ? allInputs.indexOf(container) : -1;
    if (currentIdx !== -1 && currentIdx + 1 < allInputs.length) {
      const nextInput = allInputs[currentIdx + 1];
      const cmContent = nextInput.querySelector<HTMLElement>(".cm-content");
      cmContent?.focus();
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
    </div>
    <CellSuggestions cell={cell} />
    </>
  );
}
