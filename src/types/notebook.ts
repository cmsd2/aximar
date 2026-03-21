export interface CellOutput {
  textOutput: string;
  latex: string | null;
  plotSvg: string | null;
  error: string | null;
  isError: boolean;
  durationMs: number;
}

export type CellStatus = "idle" | "running" | "queued" | "error" | "success";

export interface Cell {
  id: string;
  input: string;
  output: CellOutput | null;
  status: CellStatus;
}

export interface Notebook {
  cells: Cell[];
}
