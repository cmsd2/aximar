import { useEffect, useState, useCallback } from "react";
import { useLingui } from "@lingui/react/macro";
import { useActiveTab } from "../store/notebookStore";
import { PanelHeader } from "./ui/PanelHeader";
import { Button } from "./ui/Button";
import { IconX } from "./ui/icons";
import { useUiStore } from "../store/uiStore";
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
      <PanelHeader
        className="variable-panel-header"
        title={t({ message: "Variables", comment: "Variable panel: the title of the panel that lists the variables bound in the Maxima session" })}
        actions={
          <>
            {loading && <span className="variable-loading">...</span>}
            {variables.length > 0 && (
              <Button size="sm" variant="secondary" onClick={handleKillAll}>
                {t({
                  message: "Kill All",
                  comment: "Variable panel button: unbinds every variable, like Maxima's kill(all). max-length:12",
                })}
              </Button>
            )}
          </>
        }
        onClose={() => useUiStore.getState().setVariablesOpen(false)}
        closeLabel={t({ message: "Close variables", comment: "Variable panel: tooltip of the x button that closes the panel" })}
      />
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
                aria-label={`kill(${name})`}
              >
                <IconX size={14} stroke={1.75} aria-hidden="true" />
              </button>
            </span>
          ))
        )}
      </div>
    </div>
  );
}
