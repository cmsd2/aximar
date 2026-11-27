import { useCallback, useEffect, useState } from "react";
import { useLingui } from "@lingui/react/macro";
import { useActiveTab, useNotebookStore } from "../store/notebookStore";
import { Cell } from "./Cell";
import { MarkdownCell } from "./MarkdownCell";
import { selectHowOf } from "../lib/selection-utils";
import { RawCell } from "./RawCell";
import { TrustBanner } from "./TrustBanner";
import { CellGap } from "./CellGap";
import { ContextMenu, type ContextMenuItem } from "./ui/ContextMenu";
import { COMMANDS, isEnabled, readCommandState } from "../commands/registry";
import { formatShortcut } from "../commands/shortcut";
import { stateFor, useRunCommand } from "../commands/useCommands";

/** The cell commands, in the context menu's order */
const CELL_COMMANDS = COMMANDS.filter((c) => c.context !== undefined).sort((a, b) => a.context! - b.context!);

interface NotebookProps {
  onViewDocs?: (name: string) => void;
}

export function Notebook({ onViewDocs }: NotebookProps) {
  const { t, i18n } = useLingui();
  const tab = useActiveTab();
  const cells = tab.cells;
  const selectedCellIds = tab.selectedCellIds;
  const selectCell = useNotebookStore((s) => s.selectCell);
  const clearSelection = useNotebookStore((s) => s.clearSelection);
  const setInsertionPoint = useNotebookStore((s) => s.setInsertionPoint);
  const runCommand = useRunCommand();
  const [menu, setMenu] = useState<{ x: number; y: number; cellId: string } | null>(null);
  const closeMenu = useCallback(() => setMenu(null), []);

  // A right-click on a cell, outside its editor's text, opens the cell
  // commands for that cell; in the text the system's menu stays
  const openMenu = useCallback((e: React.MouseEvent, cellId: string) => {
    const target = e.target as HTMLElement;
    if (target.closest(".cm-content, .markdown-cell-textarea, input, textarea")) return;
    e.preventDefault();
    setMenu({ x: e.clientX, y: e.clientY, cellId });
  }, []);

  const menuItems = (cellId: string): ContextMenuItem[] => {
    const state = stateFor(readCommandState(), cellId);
    let group = -1;
    return CELL_COMMANDS.map((c) => {
      // Run, insert, move and delete each make a group
      const g = c.id.startsWith("cell.run") ? 0 : c.id.startsWith("cell.new") ? 1 : c.id.startsWith("cell.move") ? 2 : 3;
      const separatorBefore = group !== -1 && g !== group;
      group = g;
      const key = c.shortcut ?? c.editorShortcut;
      return {
        id: c.id,
        label: i18n._(c.contextLabel ?? c.label),
        shortcut: key ? formatShortcut(key) : undefined,
        disabled: !isEnabled(c, state),
        separatorBefore,
        onChoose: () => void runCommand(c.id, cellId),
      };
    });
  };

  const handleBracketClick = useCallback(
    (e: React.MouseEvent, cellId: string) => {
      e.stopPropagation();
      selectCell(cellId, selectHowOf(e));
    },
    [selectCell],
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
    <div className={`notebook${cells.length === 0 ? " empty" : ""}`} onClick={handleNotebookClick}>
      <TrustBanner />
      <CellGap after={null} />
      {cells.map((cell) => {
        const isSelected = selectedCellIds.includes(cell.id);
        const selectBracket = (
          <div
            className={`cell-select-bracket${isSelected ? " selected" : ""}`}
            onClick={(e) => handleBracketClick(e, cell.id)}
            title={
              isSelected
                ? t({
                    message: "Selected (Cmd+click to deselect)",
                    comment: "Tooltip of the place that selects a cell, on a selected cell: the bracket right of it in bracket style, the margin with its In and Out labels in card style. Cmd+click removes it from the selection",
                  })
                : t({
                    message: "Select cell (Shift+click for a range, Cmd+click to add)",
                    comment: "Tooltip of the place that selects a cell: the bracket right of it in bracket style, the margin with its In and Out labels in card style. Shift+click selects every cell up to it; Cmd+click adds it to the selection",
                  })
            }
          />
        );
        return (
          <div key={cell.id} onContextMenu={(e) => openMenu(e, cell.id)}>
            {cell.cellType === "markdown" ? (
              <MarkdownCell cell={cell} selectBracket={selectBracket} />
            ) : cell.cellType === "raw" ? (
              <RawCell cell={cell} selectBracket={selectBracket} />
            ) : (
              <Cell cell={cell} onViewDocs={onViewDocs} selectBracket={selectBracket} />
            )}
            <CellGap after={cell.id} />
          </div>
        );
      })}
      {/* The space below the last cell: a click there places the caret
          after it, as in an empty notebook, where it fills the window */}
      <div
        className="notebook-tail"
        onMouseDown={(e) => {
          e.preventDefault();
          setInsertionPoint({ after: cells[cells.length - 1]?.id ?? null });
        }}
      />
      {menu && <ContextMenu x={menu.x} y={menu.y} items={menuItems(menu.cellId)} onClose={closeMenu} />}
    </div>
  );
}
