/** Where a message's translation is found: its catalog context and English template */
export interface MessageSource {
  id: string;
  template: string;
}

/** Aximar's explanation of an error, in English (see aximar-core's `errors.rs`) */
export interface ErrorInfo {
  /** Such as `error.division_by_zero`; missing from before IDs */
  id?: string;
  /** Every message's parameters, by placeholder name */
  params?: Record<string, string>;
  /** Each translatable field's source: `title`, `explanation`, `suggestion` */
  messages?: Record<string, MessageSource>;
  title: string;
  explanation: string;
  suggestion: string | null;
  example: string | null;
  did_you_mean: string[];
  correct_signatures: string[];
}

export interface EvalResult {
  cell_id: string;
  text_output: string;
  latex: string | null;
  plot_svg: string | null;
  plot_data: string | null;
  image_png: string | null;
  error: string | null;
  error_info: ErrorInfo | null;
  is_error: boolean;
  duration_ms: number;
  /** Maxima output label (e.g. "%o6") for stable back-references */
  output_label: string | null;
}

export type SessionStatus =
  | "Starting"
  | "Ready"
  | "Busy"
  | "Stopped"
  | { Error: string };
