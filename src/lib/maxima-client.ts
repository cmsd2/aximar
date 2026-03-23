import { invoke } from "@tauri-apps/api/core";
import type { EvalResult, SessionStatus } from "../types/maxima";

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
    cellId,
    expression,
    labelContext: labelContext ?? null,
  });
}

export async function startSession(): Promise<SessionStatus> {
  return invoke<SessionStatus>("start_session");
}

export async function stopSession(): Promise<SessionStatus> {
  return invoke<SessionStatus>("stop_session");
}

export async function restartSession(): Promise<SessionStatus> {
  return invoke<SessionStatus>("restart_session");
}

export async function getSessionStatus(): Promise<SessionStatus> {
  return invoke<SessionStatus>("get_session_status");
}
