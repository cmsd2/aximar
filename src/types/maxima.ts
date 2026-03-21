export interface EvalResult {
  cell_id: string;
  text_output: string;
  latex: string | null;
  plot_svg: string | null;
  error: string | null;
  is_error: boolean;
  duration_ms: number;
}

export type SessionStatus =
  | "Starting"
  | "Ready"
  | "Busy"
  | "Stopped"
  | { Error: string };
