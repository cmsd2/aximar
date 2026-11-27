import { useState, useRef, useEffect, useCallback } from "react";
import ReactMarkdown from "react-markdown";
import { useLingui } from "@lingui/react/macro";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import rehypeKatex from "rehype-katex";
import type { Cell as CellType } from "../types/notebook";
import { useActiveTab, useNotebookStore } from "../store/notebookStore";
import { useFindStore } from "../store/findStore";
import { nbDeleteCell } from "../lib/notebook-commands";

interface MarkdownCellProps {
  cell: CellType;
  selectBracket?: React.ReactNode;
}

export function MarkdownCell({ cell, selectBracket }: MarkdownCellProps) {
  const { t } = useLingui();
  const [editing, setEditing] = useState(false);
  const cellRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const updateCellInput = useNotebookStore((s) => s.updateCellInput);
  const cells = useActiveTab().cells;
  const cellCount = cells.length;
  const hasFindMatch = useFindStore((s) => s.matches.some((m) => m.cellId === cell.id));

  useEffect(() => {
    if (editing && textareaRef.current) {
      const ta = textareaRef.current;
      ta.focus();
      ta.style.height = "auto";
      ta.style.height = ta.scrollHeight + "px";
    }
  }, [editing]);

  const handleBlur = useCallback(() => {
    setEditing(false);
  }, []);

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLTextAreaElement>) => {
      updateCellInput(cell.id, e.target.value);
      const ta = e.target;
      ta.style.height = "auto";
      ta.style.height = ta.scrollHeight + "px";
    },
    [cell.id, updateCellInput]
  );

  const focusNextCell = useCallback(() => {
    const allCells = Array.from(
      document.querySelectorAll<HTMLElement>(".cell")
    );
    const thisCell = cellRef.current;
    const idx = thisCell ? allCells.indexOf(thisCell) : -1;
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

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
      if (e.key === "Escape") {
        e.preventDefault();
        setEditing(false);
      } else if (e.key === "Enter" && e.shiftKey) {
        e.preventDefault();
        setEditing(false);
        requestAnimationFrame(focusNextCell);
      }
    },
    [focusNextCell]
  );

  return (
    <div ref={cellRef} className={`cell markdown-cell${editing ? " editing" : ""}${hasFindMatch ? " has-find-match" : ""}`}>
      {editing ? (
        <div className="markdown-cell-edit">
          <textarea
            ref={textareaRef}
            className="markdown-cell-textarea"
            value={cell.input}
            onChange={handleChange}
            onBlur={handleBlur}
            onKeyDown={handleKeyDown}
            placeholder={t({
              message: "Enter markdown...",
              comment: "Placeholder in the empty text box of a markdown cell being edited",
            })}
            rows={3}
          />
        </div>
      ) : (
        <div
          className="markdown-cell-view"
          onDoubleClick={() => setEditing(true)}
        >
          {cell.input.trim() ? (
            <ReactMarkdown remarkPlugins={[remarkGfm, remarkMath]} rehypePlugins={[rehypeKatex]}>
              {cell.input}
            </ReactMarkdown>
          ) : (
            <p className="markdown-cell-placeholder">
              {t({
                message: "Double-click to edit markdown...",
                comment: "Shown in an empty markdown cell that is not being edited",
              })}
            </p>
          )}
        </div>
      )}
      <div className="cell-actions">
        <button
          className={`cell-btn ${editing ? "run-btn" : "edit-btn"}`}
          onMouseDown={(e) => {
            e.preventDefault();
            e.stopPropagation();
            setEditing(!editing);
          }}
          title={
            editing
              ? t({
                  message: "Done editing (Escape)",
                  comment: "Tooltip of the tick button on a markdown cell being edited: shows the rendered text. Escape is the key; keep it as it is",
                })
              : t({
                  message: "Edit markdown",
                  comment: "Tooltip of the pencil button on a markdown cell: opens its text for editing",
                })
          }
        >
          {editing ? "\u2713" : "\u270E"}
        </button>
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
      {selectBracket}
    </div>
  );
}
