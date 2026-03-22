import { useRef, useCallback, useState, useEffect } from "react";
import type { Cell as CellType } from "../types/notebook";
import { useNotebookStore } from "../store/notebookStore";
import { useFindStore } from "../store/findStore";
import { useMaxima } from "../hooks/useMaxima";
import { useAutocomplete } from "../hooks/useAutocomplete";
import { useHoverTooltip } from "../hooks/useHoverTooltip";
import { useSignatureHint } from "../hooks/useSignatureHint";
import { useSnippet } from "../hooks/useSnippet";
import { findEnclosingCall } from "../lib/param-tracker";
import { CellOutput } from "./CellOutput";
import { CellSuggestions } from "./CellSuggestions";
import { AutocompletePopup } from "./AutocompletePopup";
import { HoverTooltip } from "./HoverTooltip";
import { SignatureHint } from "./SignatureHint";

interface CellProps {
  cell: CellType;
  onViewDocs?: (name: string) => void;
}

export function Cell({ cell, onViewDocs }: CellProps) {
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const updateCellInput = useNotebookStore((s) => s.updateCellInput);
  const deleteCell = useNotebookStore((s) => s.deleteCell);
  const moveCell = useNotebookStore((s) => s.moveCell);
  const addCell = useNotebookStore((s) => s.addCell);
  const cells = useNotebookStore((s) => s.cells);
  const cellCount = cells.length;
  const setActiveCellId = useNotebookStore((s) => s.setActiveCellId);
  const autocompleteMode = useNotebookStore((s) => s.autocompleteMode);
  const { executeCell } = useMaxima();

  const signatureHint = useSignatureHint(
    textareaRef,
    autocompleteMode === "active-hint" ? "active-hint" : "hint"
  );
  const snippet = useSnippet(textareaRef);

  const handleAcceptCompletion = useCallback(
    ({ funcName, insertPosition }: { funcName: string; insertPosition: number }) => {
      signatureHint.dismiss();
      snippet.exit();
      if (autocompleteMode === "snippet") {
        snippet.activate(funcName, insertPosition);
      } else {
        signatureHint.show(funcName, insertPosition);
      }
    },
    [autocompleteMode, signatureHint, snippet]
  );

  const autocomplete = useAutocomplete(textareaRef, handleAcceptCompletion);
  const hoverTooltip = useHoverTooltip(
    textareaRef,
    autocomplete.state.visible || signatureHint.state.visible || snippet.state.active
  );
  const [, setAutocompleteIndex] = useState(0);
  const [outputCollapsed, setOutputCollapsed] = useState(false);
  const hasFindMatch = useFindStore((s) => s.matches.some((m) => m.cellId === cell.id));

  // Auto-resize textarea when input changes (including initial load from templates)
  useEffect(() => {
    const textarea = textareaRef.current;
    if (textarea) {
      textarea.style.height = "auto";
      textarea.style.height = textarea.scrollHeight + "px";
    }
  }, [cell.input]);

  // Dismiss hint/snippet when mode changes
  useEffect(() => {
    signatureHint.dismiss();
    snippet.exit();
  }, [autocompleteMode]); // eslint-disable-line react-hooks/exhaustive-deps

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
      // 1. Let autocomplete handle keys first
      if (autocomplete.handleKeyDown(e)) {
        return;
      }

      // 2. Snippet Tab/Shift+Tab/Escape
      if (snippet.state.active && snippet.handleKeyDown(e)) {
        return;
      }

      // 3. Signature hint Escape
      if (signatureHint.state.visible && signatureHint.handleKeyDown(e)) {
        return;
      }

      // 4. Cell shortcuts
      if (e.key === "Enter" && e.shiftKey) {
        e.preventDefault();
        signatureHint.dismiss();
        snippet.exit();
        const idx = cells.findIndex((c) => c.id === cell.id);
        const isLastCell = idx === cells.length - 1;

        executeCell(cell.id, cell.input).then((success) => {
          if (!success) return;
          if (isLastCell) {
            addCell(cell.id);
          }
          requestAnimationFrame(focusNextCell);
        });
      } else if (e.key === "Enter" && (e.ctrlKey || e.metaKey)) {
        e.preventDefault();
        signatureHint.dismiss();
        snippet.exit();
        executeCell(cell.id, cell.input);
      } else if (e.key === "Escape") {
        e.preventDefault();
        textareaRef.current?.blur();
      }
    },
    [cell.id, cell.input, cells, executeCell, addCell, focusNextCell, autocomplete, snippet, signatureHint]
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
      // Update snippet placeholders
      snippet.handleInput();

      const text = textarea.value;
      const cursorPos = textarea.selectionStart;

      // Detect "(" typed after an identifier → trigger argument help
      if (
        cursorPos > 1 &&
        text[cursorPos - 1] === "(" &&
        !signatureHint.state.visible &&
        !snippet.state.active
      ) {
        const call = findEnclosingCall(text, cursorPos);
        if (call) {
          if (autocompleteMode === "snippet") {
            snippet.activate(call.funcName, call.openParenPos);
          } else {
            signatureHint.show(call.funcName, call.openParenPos);
          }
        }
      }

      // Update signature hint (active-hint tracks cursor)
      if (signatureHint.state.visible && autocompleteMode === "active-hint") {
        signatureHint.update();
      }
      // For hint mode, dismiss on ")" typed
      if (signatureHint.state.visible && autocompleteMode === "hint") {
        if (cursorPos > 0 && text[cursorPos - 1] === ")") {
          signatureHint.dismiss();
        }
      }
      // Hide hover tooltip on typing
      hoverTooltip.hide();
    },
    [cell.id, updateCellInput, autocomplete, hoverTooltip, snippet, signatureHint, autocompleteMode]
  );

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
        <textarea
          ref={textareaRef}
          className="cell-input"
          data-cell-id={cell.id}
          value={cell.input}
          onChange={handleChange}
          onKeyDown={handleKeyDown}
          onFocus={() => {
            setActiveCellId(cell.id);
            // Detect if cursor lands inside a function call (e.g. from command palette insert)
            requestAnimationFrame(() => {
              const textarea = textareaRef.current;
              if (!textarea || signatureHint.state.visible || snippet.state.active) return;
              const text = textarea.value;
              const cursorPos = textarea.selectionStart;
              if (cursorPos > 1) {
                const call = findEnclosingCall(text, cursorPos);
                if (call) {
                  if (autocompleteMode === "snippet") {
                    snippet.activate(call.funcName, call.openParenPos);
                  } else {
                    signatureHint.show(call.funcName, call.openParenPos);
                  }
                }
              }
            });
          }}
          onBlur={() => {
            // Delay dismiss so popup click can fire
            setTimeout(() => {
              autocomplete.dismiss();
              signatureHint.dismiss();
              snippet.exit();
            }, 150);
          }}
          onMouseMove={hoverTooltip.onMouseMove}
          onMouseLeave={hoverTooltip.onMouseLeave}
          placeholder="Enter Maxima expression... (Shift+Enter to evaluate)"
          rows={1}
          spellCheck={false}
        />
        <div className="cell-actions">
          {cellCount > 1 && (
            <>
              <button
                className="cell-btn move-btn"
                onClick={() => moveCell(cell.id, "up")}
                title="Move cell up"
                disabled={cells[0]?.id === cell.id}
              >
                &#9650;
              </button>
              <button
                className="cell-btn move-btn"
                onClick={() => moveCell(cell.id, "down")}
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
      <SignatureHint state={signatureHint.state} textareaRef={textareaRef} />
      {hoverTooltip.state.visible && hoverTooltip.state.func && onViewDocs && (
        <HoverTooltip
          func={hoverTooltip.state.func}
          x={hoverTooltip.state.x}
          y={hoverTooltip.state.y}
          onViewDocs={onViewDocs}
          onMouseEnter={hoverTooltip.cancelHide}
          onMouseLeave={hoverTooltip.scheduleHide}
        />
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
    </div>
    <CellSuggestions cell={cell} />
    </>
  );
}
