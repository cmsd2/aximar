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
 * Find the real Maxima output label for the cell immediately before cellId
 * that has output, or the most recently evaluated cell if running individually.
 */
function findPreviousOutputLabel(cellId: string): string | null {
  const cells = useNotebookStore.getState().cells;
  const idx = cells.findIndex((c) => c.id === cellId);
  // Walk backwards from the cell before this one
  for (let i = idx - 1; i >= 0; i--) {
    const label = cells[i].output?.outputLabel;
    if (label) return label;
  }
  return null;
}

/**
 * Rewrite %oN, %iN, and bare % references in an expression so they map
 * to real Maxima output/input labels. Bare % (meaning "last output") is
 * replaced with the output label of the previous cell, since our protocol
 * commands (tex, print sentinel) clobber Maxima's internal %.
 */
function rewriteLabels(input: string, labelMap: Map<number, string>, cellId: string): string {
  // First, replace bare % with the previous cell's real output label.
  // Match % not followed by a letter, digit, or another % (to avoid
  // clobbering %e, %pi, %i, %o1, %th, %%, etc.)
  const prevLabel = findPreviousOutputLabel(cellId);
  let result = input;
  if (prevLabel) {
    result = result.replace(/%(?![%a-zA-Z0-9_])/g, prevLabel);
  }

  // Then, replace display %oN/%iN with real Maxima labels
  result = result.replace(/%([oi])(\d+)/g, (match, kind: string, numStr: string) => {
    const num = parseInt(numStr, 10);
    const realLabel = labelMap.get(num);
    if (!realLabel) return match;
    const realNum = realLabel.replace("%o", "");
    return `%${kind}${realNum}`;
  });

  return result;
}

/**
 * Build a map from real Maxima output label number → the LaTeX for that output.
 * Used to inline previous results when Maxima output references %oN.
 */
function buildLabelLatexMap(): Map<number, string> {
  const cells = useNotebookStore.getState().cells;
  const map = new Map<number, string>();
  for (const cell of cells) {
    const label = cell.output?.outputLabel;
    const latex = cell.output?.latex;
    if (label != null && latex != null) {
      const realNum = parseInt(label.replace("%o", ""), 10);
      if (!isNaN(realNum)) {
        map.set(realNum, latex);
      }
    }
  }
  return map;
}

/**
 * If the text output is a bare %oN reference (Maxima returned a previous
 * result unchanged), return the LaTeX of the referenced expression.
 * Returns null if the text output is not a label reference.
 */
function resolveOutputLabel(textOutput: string, latexMap: Map<number, string>): string | null {
  const match = textOutput.trim().match(/^%o(\d+)$/);
  if (!match) return null;
  const realNum = parseInt(match[1], 10);
  return latexMap.get(realNum) ?? null;
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

      // Translate display %oN/%iN and bare % to real Maxima labels
      const labelMap = buildLabelMap();
      const rewritten = rewriteLabels(input, labelMap, cellId);

      try {
        const result = await evaluateExpression(cellId, rewritten);
        // If Maxima returned a bare %oN reference (expression unchanged),
        // substitute the referenced cell's LaTeX instead of showing the label
        const latexMap = buildLabelLatexMap();
        const resolvedLatex = resolveOutputLabel(result.text_output, latexMap);
        const output: CellOutput = {
          textOutput: result.text_output,
          latex: resolvedLatex ?? result.latex,
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
