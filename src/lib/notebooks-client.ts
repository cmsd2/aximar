import { invoke } from "@tauri-apps/api/core";
import { save, open } from "@tauri-apps/plugin-dialog";
import type { Notebook, NotebookCell, NotebookMetadata } from "../types/notebooks";
import type { TemplateSummary } from "../types/notebooks";
import type { Cell } from "../types/notebook";

const NOTEBOOK_FILTERS = [
  { name: "Maxima Notebook", extensions: ["macnb"] },
  { name: "Jupyter Notebook", extensions: ["ipynb"] },
];

const ALL_OPEN_FILTERS = [
  { name: "All Supported", extensions: ["macnb", "ipynb", "json"] },
  { name: "Maxima Notebook", extensions: ["macnb"] },
  { name: "Jupyter Notebook", extensions: ["ipynb"] },
  { name: "Plotly JSON", extensions: ["json"] },
];

export async function listTemplates(): Promise<TemplateSummary[]> {
  return invoke<TemplateSummary[]>("list_templates");
}

export async function getTemplate(id: string): Promise<Notebook | null> {
  return invoke<Notebook | null>("get_template", { id });
}

export async function getHasSeenWelcome(): Promise<boolean> {
  return invoke<boolean>("get_has_seen_welcome");
}

export async function setHasSeenWelcome(): Promise<void> {
  return invoke<void>("set_has_seen_welcome");
}

/** Convert a frontend CellOutput to nbformat outputs array.
 *
 * Uses two separate output entries so the round-trip is lossless:
 * - Intermediate/print text → `stream` output (name: "stdout")
 * - Final result LaTeX → `execute_result` with `text/latex`
 * - Text-only result (no LaTeX) → `execute_result` with `text/plain`
 */
function cellOutputToNbformat(cell: Cell): unknown[] {
  if (!cell.output || cell.cellType !== "code") return [];
  const outputs: unknown[] = [];

  // Intermediate/print text as a stream output
  if (cell.output.textOutput) {
    outputs.push({
      output_type: "stream",
      name: "stdout",
      text: [cell.output.textOutput],
    });
  }

  // Final result
  if (cell.output.latex) {
    outputs.push({
      output_type: "execute_result",
      data: { "text/latex": [cell.output.latex] },
      metadata: {},
      execution_count: cell.output.executionCount ?? null,
    });
  } else if (!cell.output.textOutput) {
    // No text, no latex — nothing to save
  }

  // Plot data as display_data with custom MIME types
  if (cell.output.plotData) {
    outputs.push({
      output_type: "display_data",
      data: { "application/x-maxima-plotly": [cell.output.plotData] },
      metadata: {},
    });
  }
  if (cell.output.plotSvg) {
    outputs.push({
      output_type: "display_data",
      data: { "image/svg+xml": [cell.output.plotSvg] },
      metadata: {},
    });
  }
  if (cell.output.imagePng) {
    outputs.push({
      output_type: "display_data",
      data: { "image/png": [cell.output.imagePng] },
      metadata: {},
    });
  }

  return outputs;
}

/** Convert frontend Cell[] to Jupyter nbformat cells for saving. */
function cellsToNotebookCells(cells: Cell[]): NotebookCell[] {
  return cells.map((cell) => ({
    cell_type: cell.cellType === "markdown" ? "markdown" : "code",
    source: cell.input,
    metadata: {},
    ...(cell.cellType === "code"
      ? { execution_count: cell.output?.executionCount ?? null, outputs: cellOutputToNbformat(cell) }
      : {}),
  }));
}

/** Parse nbformat cell outputs into a simplified output object for the backend.
 *
 * In nbformat, each output entry's `data` dict holds alternative MIME
 * representations of the *same* value — text/plain is a fallback for
 * text/latex, not additional content. So within a single execute_result or
 * display_data entry, we prefer text/latex and only fall back to text/plain
 * when no text/latex exists. Stream outputs are genuinely separate content
 * (print output, intermediate results) and always go to text_output.
 */
export function parseNbformatOutputs(cell: NotebookCell): {
  text_output: string; latex: string | null; plot_data: string | null; plot_svg: string | null; image_png: string | null; execution_count: number | null;
} | null {
  if (!cell.outputs?.length) return null;
  let textOutput = "";
  let latex: string | null = null;
  let plotData: string | null = null;
  let plotSvg: string | null = null;
  let imagePng: string | null = null;
  let executionCount: number | null = cell.execution_count ?? null;

  for (const raw of cell.outputs) {
    const out = raw as Record<string, unknown>;
    const type = out.output_type as string;
    if (type === "execute_result" || type === "display_data") {
      const data = out.data as Record<string, unknown> | undefined;
      if (data) {
        const plotly = data["application/x-maxima-plotly"];
        if (plotly) plotData = Array.isArray(plotly) ? (plotly as string[]).join("") : String(plotly);
        const svg = data["image/svg+xml"];
        if (svg) plotSvg = Array.isArray(svg) ? (svg as string[]).join("") : String(svg);
        const png = data["image/png"];
        if (png) imagePng = Array.isArray(png) ? (png as string[]).join("") : String(png);
        const tex = data["text/latex"];
        if (tex) {
          // Prefer LaTeX; text/plain is just a fallback for the same value
          latex = Array.isArray(tex) ? (tex as string[]).join("") : String(tex);
        } else {
          // No LaTeX — use text/plain as text output
          const plain = data["text/plain"];
          if (plain) textOutput += Array.isArray(plain) ? (plain as string[]).join("") : String(plain);
        }
      }
      if (type === "execute_result" && out.execution_count != null) {
        executionCount = out.execution_count as number;
      }
    } else if (type === "stream") {
      const text = out.text;
      textOutput += Array.isArray(text) ? (text as string[]).join("") : String(text ?? "");
    }
  }

  if (!textOutput && !latex && !plotData && !plotSvg && !imagePng) return null;
  return { text_output: textOutput, latex, plot_data: plotData, plot_svg: plotSvg, image_png: imagePng, execution_count: executionCount };
}

function buildNotebook(cells: Cell[]): Notebook {
  const metadata: NotebookMetadata = {
    kernelspec: {
      name: "maxima",
      display_name: "Maxima",
      language: "maxima",
    },
  };
  return {
    nbformat: 4,
    nbformat_minor: 0,
    metadata,
    cells: cellsToNotebookCells(cells),
  };
}

/**
 * Save the notebook to the given path (or prompt with Save As dialog).
 * Returns the path saved to, or null if cancelled.
 */
export async function saveNotebook(
  cells: Cell[],
  filePath: string | null,
): Promise<string | null> {
  const path =
    filePath ??
    (await save({
      defaultPath: "notebook.macnb",
      filters: NOTEBOOK_FILTERS,
    }));
  if (!path) return null;

  const notebook = buildNotebook(cells);
  await invoke("save_notebook", { path, notebook });
  return path;
}

export type OpenFileResult =
  | { type: "notebook"; notebook: Notebook; path: string }
  | { type: "plot"; plotData: string; path: string };

/**
 * Show an Open dialog and load the selected file.
 * Returns a notebook or plot result, or null if cancelled.
 */
export async function openFile(): Promise<OpenFileResult | null> {
  const selected = await open({
    multiple: false,
    filters: ALL_OPEN_FILTERS,
  });
  if (!selected) return null;

  const path = typeof selected === "string" ? selected : selected;

  if (path.endsWith(".json")) {
    const plotData = await invoke<string>("read_text_file", { path });
    return { type: "plot", plotData, path };
  }

  const notebook = await invoke<Notebook>("open_notebook", { path });
  return { type: "notebook", notebook, path };
}

/**
 * Show an Open dialog and load the selected notebook.
 * Returns { notebook, path } or null if cancelled.
 * @deprecated Use openFile() which also supports plot files.
 */
export async function openNotebook(): Promise<{
  notebook: Notebook;
  path: string;
} | null> {
  const selected = await open({
    multiple: false,
    filters: NOTEBOOK_FILTERS,
  });
  if (!selected) return null;

  const path = typeof selected === "string" ? selected : selected;
  const notebook = await invoke<Notebook>("open_notebook", { path });
  return { notebook, path };
}

/** Save a Plotly JSON string to a file (always shows dialog). Returns the saved path, or null if cancelled. */
export async function savePlotFile(
  plotData: string,
  defaultPath: string | null,
): Promise<string | null> {
  const path = await save({
    defaultPath: defaultPath ?? "plot.json",
    filters: [{ name: "Plotly JSON", extensions: ["json"] }],
  });
  if (!path) return null;

  await invoke("write_text_file", { path, content: plotData });
  return path;
}

/**
 * Open a file by path (no dialog). Used for CLI file arguments.
 */
export async function openFilePath(path: string): Promise<OpenFileResult> {
  if (path.endsWith(".json")) {
    const plotData = await invoke<string>("read_text_file", { path });
    return { type: "plot", plotData, path };
  }

  const notebook = await invoke<Notebook>("open_notebook", { path });
  return { type: "notebook", notebook, path };
}

/**
 * Save As: always shows a dialog regardless of current filePath.
 * Returns the path saved to, or null if cancelled.
 */
export async function saveNotebookAs(
  cells: Cell[],
  currentPath: string | null,
): Promise<string | null> {
  const defaultPath = currentPath ?? "notebook.macnb";
  const path = await save({
    defaultPath,
    filters: NOTEBOOK_FILTERS,
  });
  if (!path) return null;

  const notebook = buildNotebook(cells);
  await invoke("save_notebook", { path, notebook });
  return path;
}
