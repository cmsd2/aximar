import { useRef, useCallback, useState, useEffect } from "react";
import type { Cell as CellType } from "../types/notebook";
import { useNotebookStore } from "../store/notebookStore";
import { useMaxima } from "../hooks/useMaxima";
import { useAutocomplete } from "../hooks/useAutocomplete";
import { CellOutput } from "./CellOutput";
import { CellSuggestions } from "./CellSuggestions";
import { AutocompletePopup } from "./AutocompletePopup";

interface CellProps {
  cell: CellType;
}

export function Cell({ cell }: CellProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const updateCellInput = useNotebookStore((s) => s.updateCellInput);
  const deleteCell = useNotebookStore((s) => s.deleteCell);
  const addCell = useNotebookStore((s) => s.addCell);
  const cells = useNotebookStore((s) => s.cells);
  const cellCount = cells.length;
  const setActiveCellId = useNotebookStore((s) => s.setActiveCellId);
  const { executeCell } = useMaxima();

  const autocomplete = useAutocomplete(textareaRef);
  const [, setAutocompleteIndex] = useState(0);
  const [outputCollapsed, setOutputCollapsed] = useState(false);

  // Auto-resize textarea when input changes (including initial load from templates)
  useEffect(() => {
    const textarea = textareaRef.current;
    if (textarea) {
      textarea.style.height = "auto";
      textarea.style.height = textarea.scrollHeight + "px";
    }
  }, [cell.input]);

  const focusNextCell = useCallback(() => {
    const allInputs = Array.from(
      document.querySelectorAll<HTMLTextAreaElement>(".cell-input")
    );
    const currentIdx = allInputs.indexOf(textareaRef.current!);
    if (currentIdx !== -1 && currentIdx + 1 < allInputs.length) {
      allInputs[currentIdx + 1].focus();
    }
  }, []);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      // Let autocomplete handle keys first
      if (autocomplete.handleKeyDown(e)) {
        return;
      }

      if (e.key === "Enter" && e.shiftKey) {
        e.preventDefault();
        const idx = cells.findIndex((c) => c.id === cell.id);
        const isLastCell = idx === cells.length - 1;

        executeCell(cell.id, cell.input).then((success) => {
          if (!success) return;
          if (isLastCell) {
            addCell(cell.id);
          }
          requestAnimationFrame(focusNextCell);
        });
      }
    },
    [cell.id, cell.input, cells, executeCell, addCell, focusNextCell, autocomplete]
  );

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      updateCellInput(cell.id, e.target.value);
      // Auto-resize textarea
      const textarea = e.target;
      textarea.style.height = "auto";
      textarea.style.height = textarea.scrollHeight + "px";
      // Trigger autocomplete
      autocomplete.handleInput();
    },
    [cell.id, updateCellInput, autocomplete]
  );

  const execNum = cell.output?.executionCount ?? null;

  return (
    <>
    <div className={`cell ${cell.status}${outputCollapsed ? " output-collapsed" : ""}`}>
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
        <textarea
          ref={textareaRef}
          className="cell-input"
          data-cell-id={cell.id}
          value={cell.input}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          onFocus={() => setActiveCellId(cell.id)}
          onBlur={() => {
            // Delay dismiss so popup click can fire
            setTimeout(() => autocomplete.dismiss(), 150);
          }}
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
      <AutocompletePopup
        state={autocomplete.state}
        textareaRef={textareaRef}
        onSelect={(i) => {
          setAutocompleteIndex(i);
          autocomplete.accept();
        }}
        onHover={(i) => setAutocompleteIndex(i)}
      />
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
