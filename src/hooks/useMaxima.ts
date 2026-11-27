import { useCallback } from "react";
import { message } from "@tauri-apps/plugin-dialog";
import { t } from "@lingui/core/macro";
import { useNotebookStore } from "../store/notebookStore";
import { useLogStore } from "../store/logStore";
import { startSession, restartSession } from "../lib/maxima-client";
import { nbRunAll, nbRunCell, type RunAllReport } from "../lib/notebook-commands";
import { flushCell, flushNotebook } from "../lib/dirty-inputs";

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

  // Commands, URLs and the variable name are placeholders, so translations
  // can't change them
  let installInstructions: string;
  switch (platform) {
    case "macos": {
      const brewCommand = "brew install maxima";
      installInstructions = t({
        message: `Install Maxima using Homebrew:\n\n  ${brewCommand}`,
        comment: "Maxima Not Found dialog, on macOS. brewCommand is the shell command that installs Maxima",
      });
      break;
    }
    case "linux": {
      const aptCommand = "sudo apt install maxima";
      const dnfCommand = "sudo dnf install maxima";
      installInstructions = t({
        message: `Install Maxima using your package manager:\n\n  Ubuntu/Debian: ${aptCommand}\n  Fedora: ${dnfCommand}`,
        comment: "Maxima Not Found dialog, on Linux. aptCommand and dnfCommand are the shell commands that install Maxima on each distribution",
      });
      break;
    }
    case "windows": {
      const downloadUrl = "https://sourceforge.net/projects/maxima/";
      installInstructions = t({
        message: `Download Maxima for Windows from:\n${downloadUrl}`,
        comment: "Maxima Not Found dialog, on Windows. downloadUrl is the web page with the installer",
      });
      break;
    }
    default: {
      const siteUrl = "https://maxima.sourceforge.io/";
      installInstructions = t({
        message: `Install Maxima from ${siteUrl}`,
        comment: "Maxima Not Found dialog, on an unknown system. siteUrl is Maxima's web site",
      });
      break;
    }
  }

  const envVar = "AXIMAR_MAXIMA_PATH";
  const notFound = t({
    message: "Aximar could not find Maxima on your system.",
    comment: "Maxima Not Found dialog: the first line, shown when Aximar can't start Maxima",
  });
  const otherLocation = t({
    message: `If Maxima is installed in a non-standard location, set the ${envVar} environment variable or configure the path in Settings.`,
    comment: "Maxima Not Found dialog: the last paragraph. envVar is the name of an environment variable. Settings is the Settings window",
  });
  const body = `${notFound}\n\n${installInstructions}\n\n${otherLocation}`;

  await message(body, {
    title: t({
      message: "Maxima Not Found",
      comment: "Title of the error dialog shown when Aximar can't find the Maxima program",
    }),
    kind: "error",
  });
}

export function useMaxima() {
  const setSessionStatus = useNotebookStore((s) => s.setSessionStatus);
  const addLog = useLogStore((s) => s.addEntry);

  const executeCell = useCallback(
    async (cellId: string, input: string): Promise<boolean> => {
      if (!input.trim()) return false;

      const preview = input.trim().split("\n")[0].slice(0, 60);
      addLog("info", `Evaluating: ${preview}`, "eval");

      try {
        // Ensure the backend has the latest input before evaluating
        const notebookId = useNotebookStore.getState().activeNotebookId;
        if (notebookId) {
          await flushCell(notebookId, cellId);
        }
        // nb_run_cell handles: set status → evaluate → set output → emit events
        // The frontend receives status/output updates via notebook-state-changed events
        const result = await nbRunCell(cellId);
        if (result.type === "needs_notebook_trust") {
          // The trust banner shows from the backend's trust_required event
          addLog("warning", `Notebook trust required: ${result.dangerous_functions.join(", ")}`, "eval");
          return false;
        }
        if (result.is_error) {
          addLog("error", `Evaluation error: ${result.error?.split("\n")[0] ?? "unknown"}`, "eval");
        } else {
          addLog("info", `Complete (${result.duration_ms}ms)`, "eval");
        }
        return !result.is_error;
      } catch (err) {
        const errMsg = String(err);
        addLog("error", `Evaluation failed: ${errMsg}`, "eval");
        return false;
      }
    },
    [addLog]
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

  const runAll = useCallback(
    async (notebookId: string): Promise<RunAllReport | null> => {
      addLog("info", "Running all cells", "eval");
      try {
        // The backend runs the cells as it holds them: send typed edits first
        await flushNotebook(notebookId);
        // Statuses, outputs and any trust banner arrive as events
        const report = await nbRunAll(notebookId);
        const ran = report.cells.filter((c) => c.status !== "not_run").length;
        if (report.stop_reason === null) {
          addLog("info", `Run All complete: ${ran} cell(s)`, "eval");
        } else {
          const stopped = report.cells.find((c) => c.cell_id === report.stopped_at);
          const detail = stopped?.error?.split("\n")[0] ?? report.stop_reason;
          addLog("warning", `Run All stopped (${report.stop_reason}): ${detail}`, "eval");
        }
        return report;
      } catch (err) {
        addLog("error", `Run All failed: ${String(err)}`, "eval");
        return null;
      }
    },
    [addLog]
  );

  return { executeCell, runAll, initSession, restartSession: doRestartSession };
}
