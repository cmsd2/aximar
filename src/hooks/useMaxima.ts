import { useCallback } from "react";
import { message } from "@tauri-apps/plugin-dialog";
import { useNotebookStore } from "../store/notebookStore";
import { useLogStore } from "../store/logStore";
import { evaluateExpression, startSession, restartSession } from "../lib/maxima-client";
import type { CellOutput } from "../types/notebook";

function isMaximaNotFoundError(errorMsg: string): boolean {
  const lower = errorMsg.toLowerCase();
  return lower.includes("no such file") || lower.includes("not found");
}

function detectPlatform(): "macos" | "linux" | "windows" | "unknown" {
  const p = navigator.platform?.toLowerCase() ?? "";
  const ua = navigator.userAgent?.toLowerCase() ?? "";
  if (p.startsWith("mac") || ua.includes("macintosh")) return "macos";
  if (p.startsWith("linux") || ua.includes("linux")) return "linux";
  if (p.startsWith("win") || ua.includes("windows")) return "windows";
  return "unknown";
}

let maximaDialogShown = false;

async function showMaximaNotFoundDialog(errorMsg: string): Promise<void> {
  if (!isMaximaNotFoundError(errorMsg)) return;
  if (maximaDialogShown) return;
  maximaDialogShown = true;

  const platform = detectPlatform();

  let installInstructions: string;
  switch (platform) {
    case "macos":
      installInstructions = "Install Maxima using Homebrew:\n\n  brew install maxima";
      break;
    case "linux":
      installInstructions =
        "Install Maxima using your package manager:\n\n" +
        "  Ubuntu/Debian: sudo apt install maxima\n" +
        "  Fedora: sudo dnf install maxima";
      break;
    case "windows":
      installInstructions =
        "Download Maxima for Windows from:\nhttps://sourceforge.net/projects/maxima/";
      break;
    default:
      installInstructions = "Install Maxima from https://maxima.sourceforge.io/";
      break;
  }

  const body =
    `Aximar could not find Maxima on your system.\n\n` +
    `${installInstructions}\n\n` +
    `If Maxima is installed in a non-standard location, set the AXIMAR_MAXIMA_PATH ` +
    `environment variable or configure the path in Settings.`;

  await message(body, { title: "Maxima Not Found", kind: "error" });
}

/**
 * Build a map from display execution count → real Maxima output label.
 * Used to translate user-facing %o1, %o2 into the real %o6, %o10 etc.
 */
function buildLabelMap(): Map<number, string> {
  const cells = useNotebookStore.getState().cells;
  const map = new Map<number, string>();
  for (const cell of cells) {
    const ec = cell.output?.executionCount;
    const label = cell.output?.outputLabel;
    if (ec != null && label != null) {
      map.set(ec, label);
    }
  }
  return map;
}

/**
 * Rewrite %oN and %iN references in an expression so the display execution
 * numbers map to real Maxima output/input labels.
 */
function rewriteLabels(input: string, labelMap: Map<number, string>): string {
  return input.replace(/%([oi])(\d+)/g, (match, kind: string, numStr: string) => {
    const num = parseInt(numStr, 10);
    const realLabel = labelMap.get(num);
    if (!realLabel) return match; // no mapping, leave as-is
    // realLabel is e.g. "%o6"; extract the number and rebuild with correct kind
    const realNum = realLabel.replace("%o", "");
    return `%${kind}${realNum}`;
  });
}

export function useMaxima() {
  const setCellStatus = useNotebookStore((s) => s.setCellStatus);
  const setCellOutput = useNotebookStore((s) => s.setCellOutput);
  const setSessionStatus = useNotebookStore((s) => s.setSessionStatus);
  const addLog = useLogStore((s) => s.addEntry);

  const executeCell = useCallback(
    async (cellId: string, input: string): Promise<boolean> => {
      if (!input.trim()) return false;

      setCellStatus(cellId, "running");
      const preview = input.trim().split("\n")[0].slice(0, 60);
      addLog("info", `Evaluating: ${preview}`, "eval");

      // Translate display %oN/%iN to real Maxima labels before evaluation
      const labelMap = buildLabelMap();
      const rewritten = rewriteLabels(input, labelMap);

      try {
        const result = await evaluateExpression(cellId, rewritten);
        const output: CellOutput = {
          textOutput: result.text_output,
          latex: result.latex,
          plotSvg: result.plot_svg,
          error: result.error,
          errorInfo: result.error_info,
          isError: result.is_error,
          durationMs: result.duration_ms,
          outputLabel: result.output_label,
          executionCount: null, // stamped by store
        };
        setCellOutput(cellId, output);
        if (result.is_error) {
          addLog("error", `Evaluation error: ${result.error?.split("\n")[0] ?? "unknown"}`, "eval");
        } else {
          addLog("info", `Complete (${result.duration_ms}ms)`, "eval");
        }
        return !result.is_error;
      } catch (err) {
        const errMsg = String(err);
        addLog("error", `Evaluation failed: ${errMsg}`, "eval");
        const output: CellOutput = {
          textOutput: "",
          latex: null,
          plotSvg: null,
          error: errMsg,
          errorInfo: null,
          isError: true,
          durationMs: 0,
          outputLabel: null,
          executionCount: null, // stamped by store
        };
        setCellOutput(cellId, output);
        return false;
      }
    },
    [setCellStatus, setCellOutput, addLog]
  );

  const initSession = useCallback(async () => {
    setSessionStatus("Starting");
    addLog("info", "Session starting...", "session");
    try {
      const status = await startSession();
      setSessionStatus(status);
      addLog("info", "Session ready", "session");
    } catch (err) {
      const errMsg = String(err);
      setSessionStatus({ Error: errMsg });
      addLog("error", `Session failed: ${errMsg}`, "session");
      await showMaximaNotFoundDialog(errMsg);
    }
  }, [setSessionStatus, addLog]);

  const doRestartSession = useCallback(async () => {
    setSessionStatus("Starting");
    addLog("info", "Session restarting...", "session");
    try {
      const status = await restartSession();
      setSessionStatus(status);
      addLog("info", "Session ready", "session");
    } catch (err) {
      const errMsg = String(err);
      setSessionStatus({ Error: errMsg });
      addLog("error", `Session restart failed: ${errMsg}`, "session");
      await showMaximaNotFoundDialog(errMsg);
    }
  }, [setSessionStatus, addLog]);

  return { executeCell, initSession, restartSession: doRestartSession };
}
