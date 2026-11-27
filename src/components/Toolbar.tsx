import { useLingui } from "@lingui/react/macro";
import { formatShortcut } from "../commands/shortcut";
import { useActiveTab } from "../store/notebookStore";
import { useMaxima } from "../hooks/useMaxima";
import { displayTitle } from "../lib/tab-title";
import { focusCell } from "../lib/cell-caret";
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
  const { t } = useLingui();
  const tab = useActiveTab();
  const activeCellId = tab.activeCellId;
  const sessionStatus = tab.sessionStatus;
  const filePath = tab.filePath;
  const isDirty = tab.isDirty;
  const canUndo = tab.canUndo;
  const canRedo = tab.canRedo;
  const { runAll: runAllCells, restartSession } = useMaxima();

  const handleAddCell = async (cellType?: string) => {
    const { cell_id } = await nbAddCell(cellType ?? "code", undefined, activeCellId ?? undefined);
    focusCell(cell_id, 0);
  };

  // The backend runs the cells and decides where to stop
  const runAll = () => runAllCells(tab.id);

  let statusText: string;
  if (sessionStatus === "Starting") {
    statusText = t({ message: "Starting", comment: "Toolbar: session status while Maxima starts up. max-length:12" });
  } else if (sessionStatus === "Ready") {
    statusText = t({ message: "Ready", comment: "Toolbar: session status when Maxima is ready to run cells. max-length:12" });
  } else if (sessionStatus === "Busy") {
    statusText = t({ message: "Busy", comment: "Toolbar: session status while Maxima runs a cell. max-length:12" });
  } else if (sessionStatus === "Stopped") {
    statusText = t({ message: "Stopped", comment: "Toolbar: session status when Maxima is not running. max-length:12" });
  } else {
    const message = sessionStatus.Error;
    statusText = t({
      message: `Error: ${message}`,
      comment: "Toolbar: session status when Maxima failed. message is the error from the backend, in English",
    });
  }

  const statusClass =
    sessionStatus === "Ready"
      ? "status-ready"
      : sessionStatus === "Starting"
        ? "status-starting"
        : "status-error";

  const fileName = displayTitle(filePath ? (filePath.split("/").pop()?.split("\\").pop() ?? "") : "");
  const undoShortcut = formatShortcut("Mod+Z");
  const redoShortcut = formatShortcut("Mod+Shift+Z");

  return (
    <div className="toolbar">
      <div className="toolbar-left">
        <button className="toolbar-btn" onClick={() => handleAddCell("code")}>
          {t({ message: "+ Cell", comment: "Toolbar button: adds a code cell below the active cell. max-length:12" })}
        </button>
        <button className="toolbar-btn" onClick={() => handleAddCell("markdown")}>
          {t({ message: "+ Markdown", comment: "Toolbar button: adds a Markdown text cell below the active cell. max-length:12" })}
        </button>
        <div className="toolbar-separator" />
        <button className="toolbar-btn" onClick={runAll}>
          {t({ message: "Run All", comment: "Toolbar button: runs every cell in order. max-length:12" })}
        </button>
        <button className="toolbar-btn" onClick={restartSession}>
          {t({ message: "Restart", comment: "Toolbar button: restarts the Maxima session, clearing its state. max-length:12" })}
        </button>
        <div className="toolbar-separator" />
        <button
          className="toolbar-btn"
          onClick={() => nbUndo()}
          disabled={!canUndo}
          title={t({
            message: `Undo (${undoShortcut})`,
            comment: "Toolbar: tooltip of the Undo button. undoShortcut is its keyboard shortcut, such as Cmd+Z",
          })}
        >
          {t({ message: "Undo", comment: "Toolbar button: undoes the last change to the notebook. max-length:12" })}
        </button>
        <button
          className="toolbar-btn"
          onClick={() => nbRedo()}
          disabled={!canRedo}
          title={t({
            message: `Redo (${redoShortcut})`,
            comment: "Toolbar: tooltip of the Redo button. redoShortcut is its keyboard shortcut, such as Cmd+Shift+Z",
          })}
        >
          {t({ message: "Redo", comment: "Toolbar button: redoes the last undone change. max-length:12" })}
        </button>
        <div className="toolbar-separator" />
        <button className="toolbar-btn" onClick={onOpenTemplates}>
          {t({ message: "Templates", comment: "Toolbar button: opens the starter notebook chooser. max-length:12" })}
        </button>
        <div className="toolbar-separator" />
        <button
          className={`toolbar-btn${variablesOpen ? " toolbar-btn-active" : ""}`}
          onClick={onToggleVariables}
        >
          {t({ message: "Variables", comment: "Toolbar button: shows or hides the variable panel. max-length:12" })}
        </button>
        <div className="toolbar-btn-wrapper">
          <button
            className={`toolbar-btn${logOpen ? " toolbar-btn-active" : ""}`}
            onClick={onToggleLog}
          >
            {t({ message: "Log", comment: "Toolbar button: shows or hides the log panel. max-length:12" })}
          </button>
          {logUnreadCount > 0 && (
            <span className="log-badge">{logUnreadCount}</span>
          )}
        </div>
        <button
          className={`toolbar-btn${docsOpen ? " toolbar-btn-active" : ""}`}
          onClick={onToggleDocs}
        >
          {t({ message: "Docs", comment: "Toolbar button: shows or hides the documentation panel. max-length:12" })}
        </button>
      </div>
      <div className="toolbar-right">
        <span className="toolbar-filename" title={filePath ?? undefined}>
          {isDirty ? `${fileName} *` : fileName}
        </span>
        <button className="toolbar-btn" onClick={onOpenSettings}>
          {t({ message: "Settings", comment: "Toolbar button: opens the settings window. max-length:12" })}
        </button>
        <span className={`session-status ${statusClass}`}>{statusText}</span>
      </div>
    </div>
  );
}
