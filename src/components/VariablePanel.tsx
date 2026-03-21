import { useEffect, useState, useCallback } from "react";
import { useNotebookStore } from "../store/notebookStore";
import {
  listVariables,
  killVariable,
  killAllVariables,
} from "../lib/variables-client";

interface VariablePanelProps {
  open: boolean;
}

export function VariablePanel({ open }: VariablePanelProps) {
  const [variables, setVariables] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const cells = useNotebookStore((s) => s.cells);
  const sessionStatus = useNotebookStore((s) => s.sessionStatus);

  const refresh = useCallback(async () => {
    if (sessionStatus !== "Ready") return;
    setLoading(true);
    try {
      const vars = await listVariables();
      setVariables(vars);
    } catch {
      // Session not ready or other error — clear list
      setVariables([]);
    } finally {
      setLoading(false);
    }
  }, [sessionStatus]);

  // Refresh when panel opens or after any cell output changes
  const outputFingerprint = cells
    .map((c) => c.output?.executionCount ?? 0)
    .join(",");

  useEffect(() => {
    if (open) {
      refresh();
    }
  }, [open, outputFingerprint, refresh]);

  const handleKill = async (name: string) => {
    try {
      await killVariable(name);
      await refresh();
    } catch {
      // ignore
    }
  };

  const handleKillAll = async () => {
    try {
      await killAllVariables();
      await refresh();
    } catch {
      // ignore
    }
  };

  if (!open) return null;

  return (
    <div className="variable-panel">
      <div className="variable-panel-header">
        <span className="variable-panel-title">Variables</span>
        {variables.length > 0 && (
          <button className="variable-kill-all-btn" onClick={handleKillAll}>
            Kill All
          </button>
        )}
        {loading && <span className="variable-loading">...</span>}
      </div>
      <div className="variable-panel-body">
        {variables.length === 0 ? (
          <span className="variable-empty">No bound variables</span>
        ) : (
          variables.map((name) => (
            <span key={name} className="variable-pill">
              <span className="variable-name">{name}</span>
              <button
                className="variable-kill-btn"
                onClick={() => handleKill(name)}
                title={`kill(${name})`}
              >
                &times;
              </button>
            </span>
          ))
        )}
      </div>
    </div>
  );
}
