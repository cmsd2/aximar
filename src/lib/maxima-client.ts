import { invoke } from "@tauri-apps/api/core";
import type { EvalResult, SessionStatus } from "../types/maxima";
import { useNotebookStore } from "../store/notebookStore";

/** Get the currently active notebook ID for passing to backend commands. */
function activeId(): string | null {
  return useNotebookStore.getState().activeNotebookId;
}

/** Labels an expression brings from a notebook the caller keeps itself:
 *  gutter number to Maxima output label, and the cell above's label. */
export interface SuppliedLabels {
  label_map: Record<number, string>;
  previous_output_label: string | null;
}

/** Evaluate an expression without a cell.  The backend prepares it as it
 *  does cells: Unicode names and label references, from the notebook's own
 *  labels or the ones given. */
export async function evaluateExpression(
  cellId: string,
  expression: string,
  labels?: SuppliedLabels,
): Promise<EvalResult> {
  return invoke<EvalResult>("evaluate_expression", {
    notebookId: activeId(),
    cellId,
    expression,
    labels: labels ?? null,
  });
}

export async function startSession(): Promise<SessionStatus> {
  return invoke<SessionStatus>("start_session", { notebookId: activeId() });
}

export async function stopSession(): Promise<SessionStatus> {
  return invoke<SessionStatus>("stop_session", { notebookId: activeId() });
}

export async function restartSession(): Promise<SessionStatus> {
  return invoke<SessionStatus>("restart_session", { notebookId: activeId() });
}

export async function getSessionStatus(): Promise<SessionStatus> {
  return invoke<SessionStatus>("get_session_status", { notebookId: activeId() });
}

/**
 * Fire a cooperative cancel at the running Maxima eval.  Effective
 * only when the AXIMAR_KERNEL_EVENTS env var was set at session
 * spawn and the cell's user code calls check_cancel() periodically.
 * Returns Ok even if the cancel transport isn't wired — UI doesn't
 * surface a hard error when the stop button is clicked.
 */
export async function cancelEvaluation(): Promise<void> {
  return invoke<void>("cancel_evaluation", { notebookId: activeId() });
}
