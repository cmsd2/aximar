import { useNotebookStore, type Theme } from "../store/notebookStore";
import { useMaxima } from "../hooks/useMaxima";

const themeLabels: Record<Theme, string> = {
  auto: "Auto",
  light: "Light",
  dark: "Dark",
};

const nextTheme: Record<Theme, Theme> = {
  auto: "light",
  light: "dark",
  dark: "auto",
};

interface ToolbarProps {
  onOpenTemplates: () => void;
  variablesOpen: boolean;
  onToggleVariables: () => void;
}

export function Toolbar({ onOpenTemplates, variablesOpen, onToggleVariables }: ToolbarProps) {
  const addCell = useNotebookStore((s) => s.addCell);
  const cells = useNotebookStore((s) => s.cells);
  const sessionStatus = useNotebookStore((s) => s.sessionStatus);
  const theme = useNotebookStore((s) => s.theme);
  const setTheme = useNotebookStore((s) => s.setTheme);
  const { executeCell, restartSession } = useMaxima();

  const runAll = async () => {
    for (const cell of cells) {
      if (cell.input.trim()) {
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

  return (
    <div className="toolbar">
      <div className="toolbar-left">
        <button className="toolbar-btn" onClick={() => addCell()}>
          + Cell
        </button>
        <button className="toolbar-btn" onClick={runAll}>
          Run All
        </button>
        <button className="toolbar-btn" onClick={restartSession}>
          Restart
        </button>
        <button className="toolbar-btn" onClick={onOpenTemplates}>
          Templates
        </button>
        <button
          className={`toolbar-btn${variablesOpen ? " toolbar-btn-active" : ""}`}
          onClick={onToggleVariables}
        >
          Variables
        </button>
      </div>
      <div className="toolbar-right">
        <button
          className="theme-btn"
          onClick={() => setTheme(nextTheme[theme])}
          title={`Theme: ${themeLabels[theme]}`}
        >
          {themeLabels[theme]}
        </button>
        <span className={`session-status ${statusClass}`}>{statusText}</span>
      </div>
    </div>
  );
}
