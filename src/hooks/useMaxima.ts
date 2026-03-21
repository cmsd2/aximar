import { useCallback } from "react";
import { useNotebookStore } from "../store/notebookStore";
import { evaluateExpression, startSession, restartSession } from "../lib/maxima-client";
import type { CellOutput } from "../types/notebook";

export function useMaxima() {
  const setCellStatus = useNotebookStore((s) => s.setCellStatus);
  const setCellOutput = useNotebookStore((s) => s.setCellOutput);
  const setSessionStatus = useNotebookStore((s) => s.setSessionStatus);

  const executeCell = useCallback(
    async (cellId: string, input: string): Promise<boolean> => {
      if (!input.trim()) return false;

      setCellStatus(cellId, "running");

      try {
        const result = await evaluateExpression(cellId, input);
        const output: CellOutput = {
          textOutput: result.text_output,
          latex: result.latex,
          plotSvg: result.plot_svg,
          error: result.error,
          isError: result.is_error,
          durationMs: result.duration_ms,
        };
        setCellOutput(cellId, output);
        return !result.is_error;
      } catch (err) {
        const output: CellOutput = {
          textOutput: "",
          latex: null,
          plotSvg: null,
          error: String(err),
          isError: true,
          durationMs: 0,
        };
        setCellOutput(cellId, output);
        return false;
      }
    },
    [setCellStatus, setCellOutput]
  );

  const initSession = useCallback(async () => {
    setSessionStatus("Starting");
    try {
      const status = await startSession();
      setSessionStatus(status);
    } catch (err) {
      setSessionStatus({ Error: String(err) });
    }
  }, [setSessionStatus]);

  const doRestartSession = useCallback(async () => {
    setSessionStatus("Starting");
    try {
      const status = await restartSession();
      setSessionStatus(status);
    } catch (err) {
      setSessionStatus({ Error: String(err) });
    }
  }, [setSessionStatus]);

  return { executeCell, initSession, restartSession: doRestartSession };
}
