import { useActiveTab } from "../store/notebookStore";
import { useMaxima } from "../hooks/useMaxima";
import { nbAddCell, nbUndo, nbRedo } from "../lib/notebook-commands";

interface ToolbarProps {
  onOpenTemplates: () => void;
  onOpenSettings: () => void;
  variablesOpen: boolean;
  onToggleVariables: () => void;
  logOpen: boolean;
  onToggleLog: () => void;
  logUnreadCount: number;
  docsOpen: boolean;
  onToggleDocs: () => void;
}

export function Toolbar({ onOpenTemplates, onOpenSettings, variablesOpen, onToggleVariables, logOpen, onToggleLog, logUnreadCount, docsOpen, onToggleDocs }: ToolbarProps) {
  const tab = useActiveTab();
  const activeCellId = tab.activeCellId;
  const cells = tab.cells;
  const sessionStatus = tab.sessionStatus;
  const filePath = tab.filePath;
  const isDirty = tab.isDirty;
  const canUndo = tab.canUndo;
  const canRedo = tab.canRedo;
  const { executeCell, restartSession } = useMaxima();

  const handleAddCell = async (cellType?: string) => {
    await nbAddCell(cellType ?? "code", undefined, activeCellId ?? undefined);
    // Focus is handled automatically: applyBackendState sets activeCellId,
    // and the CodeMirror hook auto-focuses when the cell becomes active.
  };

  const runAll = async () => {
    for (const cell of cells) {
      if (cell.cellType === "code" && cell.input.trim()) {
        const success = await executeCell(cell.id, cell.input);
        if (!success) break;
      }
    }
  };

  const statusText =
    typeof sessionStatus === "string"
      ? sessionStatus
      : `Error: ${sessionStatus.Error}`;

  const statusClass =
    sessionStatus === "Ready"
      ? "status-ready"
      : sessionStatus === "Starting"
        ? "status-starting"
        : "status-error";

  const fileName = filePath
    ? filePath.split("/").pop()?.split("\\").pop() ?? "Untitled"
    : "Untitled";

  return (
    <div className="toolbar">
      <div className="toolbar-left">
        <button className="toolbar-btn" onClick={() => handleAddCell("code")}>
          + Cell
        </button>
        <button className="toolbar-btn" onClick={() => handleAddCell("markdown")}>
          + Markdown
        </button>
        <div className="toolbar-separator" />
        <button className="toolbar-btn" onClick={runAll}>
          Run All
        </button>
        <button className="toolbar-btn" onClick={restartSession}>
          Restart
        </button>
        <div className="toolbar-separator" />
        <button className="toolbar-btn" onClick={() => nbUndo()} disabled={!canUndo} title="Undo (Cmd+Z)">
          Undo
        </button>
        <button className="toolbar-btn" onClick={() => nbRedo()} disabled={!canRedo} title="Redo (Cmd+Shift+Z)">
          Redo
        </button>
        <div className="toolbar-separator" />
        <button className="toolbar-btn" onClick={onOpenTemplates}>
          Templates
        </button>
        <div className="toolbar-separator" />
        <button
          className={`toolbar-btn${variablesOpen ? " toolbar-btn-active" : ""}`}
          onClick={onToggleVariables}
        >
          Variables
        </button>
        <div className="toolbar-btn-wrapper">
          <button
            className={`toolbar-btn${logOpen ? " toolbar-btn-active" : ""}`}
            onClick={onToggleLog}
          >
            Log
          </button>
          {logUnreadCount > 0 && (
            <span className="log-badge">{logUnreadCount}</span>
          )}
        </div>
        <button
          className={`toolbar-btn${docsOpen ? " toolbar-btn-active" : ""}`}
          onClick={onToggleDocs}
        >
          Docs
        </button>
      </div>
      <div className="toolbar-right">
        <span className="toolbar-filename" title={filePath ?? undefined}>
          {isDirty ? `${fileName} *` : fileName}
        </span>
        <button className="toolbar-btn" onClick={onOpenSettings}>
          Settings
        </button>
        <span className={`session-status ${statusClass}`}>{statusText}</span>
      </div>
    </div>
  );
}
