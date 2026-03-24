import { useCallback } from "react";
import { message } from "@tauri-apps/plugin-dialog";
import { useNotebookStore } from "../store/notebookStore";
import { useLogStore } from "../store/logStore";
import { startSession, restartSession } from "../lib/maxima-client";
import { nbRunCell } from "../lib/notebook-commands";

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

export function useMaxima() {
  const setSessionStatus = useNotebookStore((s) => s.setSessionStatus);
  const addLog = useLogStore((s) => s.addEntry);

  const executeCell = useCallback(
    async (cellId: string, input: string): Promise<boolean> => {
      if (!input.trim()) return false;

      const preview = input.trim().split("\n")[0].slice(0, 60);
      addLog("info", `Evaluating: ${preview}`, "eval");

      try {
        // nb_run_cell handles: set status → evaluate → set output → emit events
        // The frontend receives status/output updates via notebook-state-changed events
        const result = await nbRunCell(cellId);
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

  return { executeCell, initSession, restartSession: doRestartSession };
}
