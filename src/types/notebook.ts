import type { ErrorInfo } from "./maxima";

export interface CellOutput {
  textOutput: string;
  latex: string | null;
  plotSvg: string | null;
  plotData: string | null;
  error: string | null;
  errorInfo: ErrorInfo | null;
  isError: boolean;
  durationMs: number;
  /** Maxima output label (e.g. "%o6") for stable back-references in formulas */
  outputLabel: string | null;
  /** Sequential execution number for display (like Jupyter's In [1] / Out[1]) */
  executionCount: number | null;
}

export type CellStatus = "idle" | "running" | "queued" | "error" | "success" | "pending_approval";

export type CellType = "code" | "markdown";

export interface Cell {
  id: string;
  cellType: CellType;
  input: string;
  output: CellOutput | null;
  status: CellStatus;
  trusted?: boolean;
  dangerousFunctions?: string[] | null;
}

export interface Notebook {
  cells: Cell[];
}
