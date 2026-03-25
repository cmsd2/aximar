import { invoke } from "@tauri-apps/api/core";
import type { EvalResult, SessionStatus } from "../types/maxima";
import { useNotebookStore } from "../store/notebookStore";

/** Get the currently active notebook ID for passing to backend commands. */
function activeId(): string | null {
  return useNotebookStore.getState().activeNotebookId;
}

export interface LabelContext {
  label_map: Record<number, string>;
  previous_output_label: string | null;
}

export async function evaluateExpression(
  cellId: string,
  expression: string,
  labelContext?: LabelContext
): Promise<EvalResult> {
  return invoke<EvalResult>("evaluate_expression", {
    notebookId: activeId(),
    cellId,
    expression,
    labelContext: labelContext ?? null,
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
