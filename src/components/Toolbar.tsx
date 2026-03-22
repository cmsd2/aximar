import { useNotebookStore } from "../store/notebookStore";
import { useMaxima } from "../hooks/useMaxima";

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
  const addCell = useNotebookStore((s) => s.addCell);
  const addMarkdownCell = useNotebookStore((s) => s.addMarkdownCell);
  const activeCellId = useNotebookStore((s) => s.activeCellId);
  const cells = useNotebookStore((s) => s.cells);
  const sessionStatus = useNotebookStore((s) => s.sessionStatus);
  const filePath = useNotebookStore((s) => s.filePath);
  const isDirty = useNotebookStore((s) => s.isDirty);
  const { executeCell, restartSession } = useMaxima();

  const runAll = async () => {
    for (const cell of cells) {
      if (cell.cellType === "code" && cell.input.trim()) {
        await executeCell(cell.id, cell.input);
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
        <button className="toolbar-btn" onClick={() => {
          const id = addCell(activeCellId ?? undefined);
          requestAnimationFrame(() => {
            const el = document.querySelector<HTMLTextAreaElement>(`[data-cell-id="${id}"]`);
            el?.focus();
          });
        }}>
          + Cell
        </button>
        <button className="toolbar-btn" onClick={() => {
          const id = addMarkdownCell(activeCellId ?? undefined);
          requestAnimationFrame(() => {
            const el = document.querySelector<HTMLTextAreaElement>(`[data-cell-id="${id}"]`);
            el?.focus();
          });
        }}>
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
