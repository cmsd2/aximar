import { useCallback, useEffect } from "react";
import { useActiveTab, useNotebookStore } from "../store/notebookStore";
import { Cell } from "./Cell";
import { MarkdownCell } from "./MarkdownCell";
import { nbAddCell } from "../lib/notebook-commands";

function InsertBar({ afterId, beforeId }: { afterId?: string; beforeId?: string }) {
  const handleAddCode = async () => {
    await nbAddCell("code", undefined, afterId, beforeId);
  };
  const handleAddMarkdown = async () => {
    await nbAddCell("markdown", undefined, afterId, beforeId);
  };

  return (
    <div className="insert-bar">
      <div className="insert-bar-line" />
      <div className="insert-bar-buttons">
        <button className="insert-bar-btn" onClick={handleAddCode}>
          + Code
        </button>
        <button className="insert-bar-btn" onClick={handleAddMarkdown}>
          + Markdown
        </button>
      </div>
      <div className="insert-bar-line" />
    </div>
  );
}

interface NotebookProps {
  onViewDocs?: (name: string) => void;
}

export function Notebook({ onViewDocs }: NotebookProps) {
  const tab = useActiveTab();
  const cells = tab.cells;
  const selectedCellIds = tab.selectedCellIds;
  const toggleCellSelected = useNotebookStore((s) => s.toggleCellSelected);
  const clearSelection = useNotebookStore((s) => s.clearSelection);

  const handleBracketClick = useCallback(
    (e: React.MouseEvent, cellId: string) => {
      e.stopPropagation();
      toggleCellSelected(cellId, e.shiftKey);
    },
    [toggleCellSelected],
  );

  // Click on the notebook background (not on a bracket) clears selection
  const handleNotebookClick = useCallback(
    (e: React.MouseEvent) => {
      const target = e.target as HTMLElement;
      if (!target.closest(".cell-select-bracket") && !target.closest(".bracket-toggle")) {
        clearSelection();
      }
    },
    [clearSelection],
  );

  // Escape key clears selection
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape" && selectedCellIds.length > 0) {
        clearSelection();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [selectedCellIds.length, clearSelection]);

  return (
    <div className="notebook" onClick={handleNotebookClick}>
      <InsertBar beforeId={cells[0]?.id} />
      {cells.map((cell) => {
        const isSelected = selectedCellIds.includes(cell.id);
        return (
          <div key={cell.id}>
            {cell.cellType === "markdown" ? (
              <MarkdownCell
                cell={cell}
                selectBracket={
                  <div
                    className={`cell-select-bracket${isSelected ? " selected" : ""}`}
                    onClick={(e) => handleBracketClick(e, cell.id)}
                    title={isSelected ? "Deselect cell" : "Select cell (Shift+click for range)"}
                  />
                }
              />
            ) : (
              <Cell
                cell={cell}
                onViewDocs={onViewDocs}
                selectBracket={
                  <div
                    className={`cell-select-bracket${isSelected ? " selected" : ""}`}
                    onClick={(e) => handleBracketClick(e, cell.id)}
                    title={isSelected ? "Deselect cell" : "Select cell (Shift+click for range)"}
                  />
                }
              />
            )}
            <InsertBar afterId={cell.id} />
          </div>
        );
      })}
    </div>
  );
}
