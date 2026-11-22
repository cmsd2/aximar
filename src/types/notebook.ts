import type { ErrorInfo } from "./maxima";

export interface CellOutput {
  textOutput: string;
  latex: string | null;
  plotSvg: string | null;
  plotData: string | null;
  imagePng: string | null;
  error: string | null;
  errorInfo: ErrorInfo | null;
  isError: boolean;
  durationMs: number;
  /** Maxima output label (e.g. "%o6") for stable back-references in formulas */
  outputLabel: string | null;
  /** Sequential execution number for display (like Jupyter's In [1] / Out[1]) */
  executionCount: number | null;
  /** Maxima's one-line form of the result, as string() writes it */
  textPlain: string | null;
  /** Read from a file: from an earlier session, until the cell runs again */
  stale: boolean;
}

export type CellStatus = "idle" | "running" | "queued" | "error" | "success";

/** Raw cells come from Jupyter files: read-only text, never run. */
export type CellType = "code" | "markdown" | "raw";

export interface Cell {
  id: string;
  cellType: CellType;
  input: string;
  output: CellOutput | null;
  status: CellStatus;
}

export interface Notebook {
  cells: Cell[];
}
