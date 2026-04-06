import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import Plotly from "plotly.js-dist-min";
import { useActiveTab, useNotebookStore } from "../store/notebookStore";
import { useMaxima } from "../hooks/useMaxima";
import { getSuggestions } from "../lib/suggestions-client";
import { nbAddCell } from "../lib/notebook-commands";
import type { Suggestion } from "../types/suggestions";
import type { Cell } from "../types/notebook";
import type { EvalResult } from "../types/maxima";

interface CellSuggestionsProps {
  cell: Cell;
}

export function CellSuggestions({ cell }: CellSuggestionsProps) {
  const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
  const updateCellInput = useNotebookStore((s) => s.updateCellInput);
  const cells = useActiveTab().cells;
  const { executeCell } = useMaxima();

  // Track the cell created by the last suggestion click so we can reuse it
  const lastSuggestionRef = useRef<{ cellId: string; template: string } | null>(null);

  const handleAction = useCallback(
    async (action: string) => {
      if (action === "save_svg" && cell.output?.plotSvg) {
        const path = await save({
          defaultPath: "plot.svg",
          filters: [{ name: "SVG Image", extensions: ["svg"] }],
        });
        if (path) {
          await invoke("write_plot_svg", { path, content: cell.output.plotSvg });
        }
      } else if (
        (action === "save_plotly_svg" || action === "save_plotly_png") &&
        cell.output?.plotData
      ) {
        // Find the Plotly chart DOM element for this cell
        const inputEl = document.querySelector(`[data-cell-id="${cell.id}"]`);
        const cellEl = inputEl?.closest(".cell");
        const plotEl = cellEl?.querySelector(".plotly-output") as HTMLElement | null;
        if (!plotEl) return;

        const isSvg = action === "save_plotly_svg";
        const format = isSvg ? "svg" : "png";
        const ext = format;

        const path = await save({
          defaultPath: `plot.${ext}`,
          filters: [{ name: `${ext.toUpperCase()} Image`, extensions: [ext] }],
        });
        if (!path) return;

        const imgData = await Plotly.toImage(plotEl, {
          format,
          width: plotEl.clientWidth || 800,
          height: plotEl.clientHeight || 500,
        });

        if (isSvg) {
          // SVG: data URL → text content
          const svgContent = imgData.startsWith("data:")
            ? decodeURIComponent(imgData.split(",")[1])
            : imgData;
          await invoke("write_plot_svg", { path, content: svgContent });
        } else {
          // PNG: data URL → binary via base64
          const base64 = imgData.split(",")[1];
          const bytes = Uint8Array.from(atob(base64), (c) => c.charCodeAt(0));
          await invoke("write_binary_file", { path, data: Array.from(bytes) });
        }
      }
    },
    [cell.id, cell.output],
  );

  useEffect(() => {
    if (!cell.output || cell.output.isError || cell.status !== "success") {
      setSuggestions([]);
      return;
    }

    const output: EvalResult = {
      cell_id: cell.id,
      text_output: cell.output.textOutput,
      latex: cell.output.latex,
      plot_svg: cell.output.plotSvg,
      plot_data: cell.output.plotData,
      error: cell.output.error,
      error_info: cell.output.errorInfo,
      is_error: cell.output.isError,
      duration_ms: cell.output.durationMs,
      output_label: cell.output.outputLabel,
    };

    getSuggestions(cell.input, output)
      .then(setSuggestions)
      .catch(() => setSuggestions([]));
  }, [cell.id, cell.input, cell.output, cell.status]);

  if (suggestions.length === 0) return null;

  const handleSuggestionClick = async (suggestion: Suggestion) => {
    const { template, position } = suggestion;
    const prev = lastSuggestionRef.current;

    // Check if we can reuse the cell from the previous suggestion click:
    // it must still exist, and its input must not have been edited by the user
    if (prev) {
      const prevCell = cells.find((c) => c.id === prev.cellId);
      if (prevCell && prevCell.input === prev.template) {
        // Reuse: update input and re-execute
        updateCellInput(prev.cellId, template);
        lastSuggestionRef.current = { cellId: prev.cellId, template };
        executeCell(prev.cellId, template);
        return;
      }
    }

    // Determine insertion point
    let afterCellId: string | undefined = cell.id;
    let beforeCellId: string | undefined;
    if (position === "before") {
      afterCellId = undefined;
      beforeCellId = cell.id;
    }

    const result = await nbAddCell("code", template, afterCellId, beforeCellId);
    lastSuggestionRef.current = { cellId: result.cell_id, template };
    executeCell(result.cell_id, template);
  };

  return (
    <div className="cell-suggestions">
      {suggestions.map((s) => (
        <button
          key={s.action ?? s.template}
          className="suggestion-chip"
          title={s.description}
          onClick={() =>
            s.action ? handleAction(s.action) : handleSuggestionClick(s)
          }
        >
          {s.label}
        </button>
      ))}
    </div>
  );
}
