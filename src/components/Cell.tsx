import { useRef, useCallback } from "react";
import type { Cell as CellType } from "../types/notebook";
import { useNotebookStore } from "../store/notebookStore";
import { useMaxima } from "../hooks/useMaxima";
import { CellOutput } from "./CellOutput";

interface CellProps {
  cell: CellType;
}

export function Cell({ cell }: CellProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const updateCellInput = useNotebookStore((s) => s.updateCellInput);
  const deleteCell = useNotebookStore((s) => s.deleteCell);
  const addCell = useNotebookStore((s) => s.addCell);
  const cellCount = useNotebookStore((s) => s.cells.length);
  const { executeCell } = useMaxima();

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Enter" && e.shiftKey) {
        e.preventDefault();
        executeCell(cell.id, cell.input);
        addCell(cell.id);
      }
    },
    [cell.id, cell.input, executeCell, addCell]
  );

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      updateCellInput(cell.id, e.target.value);
      // Auto-resize textarea
      const textarea = e.target;
      textarea.style.height = "auto";
      textarea.style.height = textarea.scrollHeight + "px";
    },
    [cell.id, updateCellInput]
  );

  return (
    <div className={`cell ${cell.status}`}>
      <div className="cell-input-area">
        <div className="cell-gutter">
          {cell.status === "running" ? (
            <span className="cell-indicator running">*</span>
          ) : (
            <span className="cell-indicator">In</span>
          )}
        </div>
        <textarea
          ref={textareaRef}
          className="cell-input"
          value={cell.input}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          placeholder="Enter Maxima expression... (Shift+Enter to evaluate)"
          rows={1}
          spellCheck={false}
        />
        <div className="cell-actions">
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
              onClick={() => deleteCell(cell.id)}
              title="Delete cell"
            >
              &times;
            </button>
          )}
        </div>
      </div>
      {cell.output && (
        <div className="cell-output-area">
          <div className="cell-gutter">
            <span className="cell-indicator">Out</span>
          </div>
          <CellOutput output={cell.output} />
        </div>
      )}
    </div>
  );
}
