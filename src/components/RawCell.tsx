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
  const cellCount = useActiveTab().cells.length;
  const hasFindMatch = useFindStore((s) => s.matches.some((m) => m.cellId === cell.id));

  return (
    <div className={`cell raw-cell${hasFindMatch ? " has-find-match" : ""}`} data-cell-id={cell.id}>
      <div className="cell-input-area">
        <div className="cell-gutter">
          <span className="cell-indicator" title="Raw cell: kept as it is, never run">
            Raw
          </span>
        </div>
        <pre className="raw-cell-text">{cell.input}</pre>
        <div className="cell-actions">
          <button
            className="cell-btn edit-btn"
            onClick={() => nbToggleCellType(cell.id)}
            title="Make this a code cell"
          >
            &#9998;
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
      {selectBracket}
    </div>
  );
}
