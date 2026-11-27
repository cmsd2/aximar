import { useEffect, useState, useCallback } from "react";
import { useLingui } from "@lingui/react/macro";
import { useActiveTab } from "../store/notebookStore";
import {
  listVariables,
  killVariable,
  killAllVariables,
} from "../lib/variables-client";

interface VariablePanelProps {
  open: boolean;
}

export function VariablePanel({ open }: VariablePanelProps) {
  const { t } = useLingui();
  const [variables, setVariables] = useState<string[]>([]);
  const [loading, setLoading] = useState(false);
  const tab = useActiveTab();
  const cells = tab.cells;
  const sessionStatus = tab.sessionStatus;

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
        <span className="variable-panel-title">
          {t({ message: "Variables", comment: "Variable panel: the title of the panel that lists the variables bound in the Maxima session" })}
        </span>
        {variables.length > 0 && (
          <button className="variable-kill-all-btn" onClick={handleKillAll}>
            {t({
              message: "Kill All",
              comment: "Variable panel button: unbinds every variable, like Maxima's kill(all). max-length:12",
            })}
          </button>
        )}
        {loading && <span className="variable-loading">...</span>}
      </div>
      <div className="variable-panel-body">
        {variables.length === 0 ? (
          <span className="variable-empty">
            {t({ message: "No bound variables", comment: "Variable panel: shown when the Maxima session has no bound variables" })}
          </span>
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
