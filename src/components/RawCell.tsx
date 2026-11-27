import { useLingui } from "@lingui/react/macro";
import type { Cell as CellType } from "../types/notebook";
import { useActiveTab } from "../store/notebookStore";
import { useFindStore } from "../store/findStore";
import { nbDeleteCell, nbToggleCellType } from "../lib/notebook-commands";

interface RawCellProps {
  cell: CellType;
  selectBracket?: React.ReactNode;
}

/**
 * A raw cell from a Jupyter file: text kept as it is and written back
 * unchanged.  It can't be edited or run; making it a code cell does both.
 */
export function RawCell({ cell, selectBracket }: RawCellProps) {
  const { t } = useLingui();
  const cellCount = useActiveTab().cells.length;
  const hasFindMatch = useFindStore((s) => s.matches.some((m) => m.cellId === cell.id));

  return (
    <div className={`cell raw-cell${hasFindMatch ? " has-find-match" : ""}`} data-cell-id={cell.id}>
      <div className="cell-input-area">
        <div className="cell-gutter">
          <span
            className="cell-indicator"
            title={t({
              message: "Raw cell: kept as it is, never run",
              comment: "Tooltip of the Raw label beside a raw cell from a Jupyter file",
            })}
          >
            {t({
              message: "Raw",
              comment: "Label left of a raw cell from a Jupyter file, in the place of In for a code cell. max-length:6",
            })}
          </span>
        </div>
        <pre className="raw-cell-text">{cell.input}</pre>
        <div className="cell-actions">
          <button
            className="cell-btn edit-btn"
            onClick={() => nbToggleCellType(cell.id)}
            title={t({
              message: "Make this a code cell",
              comment: "Tooltip of the pencil button on a raw cell: turns it into a code cell that can be edited and run",
            })}
          >
            &#9998;
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
      </div>
      {selectBracket}
    </div>
  );
}
