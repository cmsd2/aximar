import Plotly from "plotly.js-dist-min";
import type { Cell } from "../types/notebook";
import { markdownToLatex } from "./markdown-to-latex";

export interface ExportOptions {
  includeCode: boolean;
  includePlots: boolean;
}

export interface ExportResult {
  tex: string;
  images: Map<string, Uint8Array>;
}

/**
 * Generate a complete LaTeX document from a list of cells.
 *
 * Returns the `.tex` source and a map of image filename → binary data
 * for any plots that need to be saved alongside the document.
 */
export async function exportCellsAsLatex(
  cells: Cell[],
  options: ExportOptions,
): Promise<ExportResult> {
  const images = new Map<string, Uint8Array>();
  const body: string[] = [];
  let plotCounter = 0;

  for (const cell of cells) {
    if (cell.cellType === "markdown") {
      body.push(markdownToLatex(cell.input));
      body.push("");
      continue;
    }

    // Code cell
    if (options.includeCode && cell.input.trim()) {
      body.push("\\begin{verbatim}");
      body.push(cell.input);
      body.push("\\end{verbatim}");
      body.push("");
    }

    if (!cell.output || cell.output.isError) continue;

    // Plot output
    if (options.includePlots) {
      if (cell.output.plotSvg) {
        plotCounter++;
        const filename = `images/plot-${plotCounter}.svg`;
        const encoder = new TextEncoder();
        images.set(filename, encoder.encode(cell.output.plotSvg));

        body.push("\\begin{figure}[h]");
        body.push("\\centering");
        body.push(
          `\\includesvg[width=0.8\\textwidth]{${filename}}`,
        );
        body.push("\\end{figure}");
        body.push("");
      } else if (cell.output.plotData) {
        plotCounter++;
        const filename = `images/plot-${plotCounter}.png`;

        // Try to render the Plotly chart to PNG
        const pngData = await renderPlotlyToPng(cell.output.plotData, cell.id);
        if (pngData) {
          images.set(filename, pngData);
          body.push("\\begin{figure}[h]");
          body.push("\\centering");
          body.push(
            `\\includegraphics[width=0.8\\textwidth]{${filename}}`,
          );
          body.push("\\end{figure}");
          body.push("");
        }
      }
    }

    // LaTeX math output
    if (cell.output.latex) {
      body.push("\\[");
      body.push(cell.output.latex);
      body.push("\\]");
      body.push("");
    } else if (cell.output.textOutput && !cell.output.plotSvg && !cell.output.plotData) {
      // Plain text output as verbatim
      body.push("\\begin{verbatim}");
      body.push(cell.output.textOutput);
      body.push("\\end{verbatim}");
      body.push("");
    }
  }

  const packages = [
    "amsmath",
    "amssymb",
    "graphicx",
    "verbatim",
    "hyperref",
  ];
  if (images.size > 0 && Array.from(images.keys()).some((k) => k.endsWith(".svg"))) {
    packages.push("svg");
  }

  const preamble = [
    "\\documentclass{article}",
    ...packages.map((p) => `\\usepackage{${p}}`),
    "",
    "\\begin{document}",
    "",
  ];

  const postamble = ["", "\\end{document}", ""];

  const tex = [...preamble, ...body, ...postamble].join("\n");
  return { tex, images };
}

/**
 * Attempt to render a Plotly chart to PNG by temporarily creating a hidden
 * div, running Plotly.toImage on it, then cleaning up.
 */
async function renderPlotlyToPng(
  plotDataJson: string,
  cellId: string,
): Promise<Uint8Array | null> {
  try {
    // First try to find an existing rendered chart in the DOM
    const inputEl = document.querySelector(`[data-cell-id="${cellId}"]`);
    const cellEl = inputEl?.closest(".cell");
    const existingPlot = cellEl?.querySelector(".plotly-output") as HTMLElement | null;

    if (existingPlot) {
      const imgData = await Plotly.toImage(existingPlot, {
        format: "png",
        width: 800,
        height: 500,
      });
      return dataUrlToBytes(imgData);
    }

    // Fallback: create a temporary hidden element and render
    const parsed = JSON.parse(plotDataJson);
    const div = document.createElement("div");
    div.style.position = "absolute";
    div.style.left = "-9999px";
    div.style.width = "800px";
    div.style.height = "500px";
    document.body.appendChild(div);

    try {
      await Plotly.newPlot(div, parsed.data || [], parsed.layout || {});
      const imgData = await Plotly.toImage(div, {
        format: "png",
        width: 800,
        height: 500,
      });
      return dataUrlToBytes(imgData);
    } finally {
      Plotly.purge(div);
      document.body.removeChild(div);
    }
  } catch {
    return null;
  }
}

function dataUrlToBytes(dataUrl: string): Uint8Array {
  const base64 = dataUrl.split(",")[1];
  const binary = atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let i = 0; i < binary.length; i++) {
    bytes[i] = binary.charCodeAt(i);
  }
  return bytes;
}
