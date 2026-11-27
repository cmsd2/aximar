// The suggestions for a code cell's result, such as "Simplify" or "Plot",
// and what choosing one does: insert a cell after it with the
// suggestion's code and run it, or save the plot.
import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { t } from "@lingui/core/macro";
import Plotly from "plotly.js-dist-min";
import { useActiveTab, useNotebookStore } from "../store/notebookStore";
import { useMaxima } from "./useMaxima";
import { getSuggestions } from "../lib/suggestions-client";
import { savePlotFile } from "../lib/notebooks-client";
import { nbAddCell } from "../lib/notebook-commands";
import type { Suggestion } from "../types/suggestions";
import type { Cell } from "../types/notebook";
import type { EvalResult } from "../types/maxima";

export interface CellSuggestionsState {
  suggestions: Suggestion[];
  /** Insert the suggestion's cell and run it, or do its action */
  choose: (suggestion: Suggestion) => void;
}

/**
 * The suggestions for `cell`'s result; none while it has no successful
 * result, or when `enabled` is false (the Settings switch)
 */
export function useCellSuggestions(cell: Cell | undefined, enabled = true): CellSuggestionsState {
  const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
  const updateCellInput = useNotebookStore((s) => s.updateCellInput);
  const cells = useActiveTab().cells;
  const { executeCell } = useMaxima();

  // Track the cell created by the last suggestion click so we can reuse it
  const lastSuggestionRef = useRef<{ cellId: string; template: string } | null>(null);

  const handleAction = useCallback(
    async (action: string) => {
      if (!cell) return;
      if (action === "save_svg" && cell.output?.plotSvg) {
        const path = await save({
          defaultPath: "plot.svg",
          filters: [
            {
              name: t({
                message: "SVG Image",
                comment: "File type in the Save dialog when saving a plot as SVG",
              }),
              extensions: ["svg"],
            },
          ],
        });
        if (path) {
          await invoke("write_plot_svg", { path, content: cell.output.plotSvg });
        }
      } else if (action === "save_plotly_json" && cell.output?.plotData) {
        await savePlotFile(cell.output.plotData, null);
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
        const kind = ext.toUpperCase();

        const path = await save({
          defaultPath: `plot.${ext}`,
          filters: [
            {
              name: t({
                message: `${kind} Image`,
                comment: "File type in the Save dialog when saving a plot. kind is the image format, SVG or PNG",
              }),
              extensions: [ext],
            },
          ],
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
    [cell],
  );

  // Refetch when what the suggestions depend on changes, not on every
  // change to the cell object
  useEffect(() => {
    if (!enabled || !cell || !cell.output || cell.output.isError || cell.status !== "success") {
      setSuggestions([]);
      return;
    }

    const output: EvalResult = {
      cell_id: cell.id,
      text_output: cell.output.textOutput,
      latex: cell.output.latex,
      plot_svg: cell.output.plotSvg,
      plot_data: cell.output.plotData,
      image_png: cell.output.imagePng ?? null,
      error: cell.output.error,
      error_info: cell.output.errorInfo,
      is_error: cell.output.isError,
      duration_ms: cell.output.durationMs,
      output_label: cell.output.outputLabel,
    };

    getSuggestions(cell.input, output)
      .then((found) => setSuggestions(found ?? []))
      .catch(() => setSuggestions([]));
  }, [enabled, cell?.id, cell?.input, cell?.output, cell?.status]);

  const handleSuggestionClick = async (suggestion: Suggestion) => {
    if (!cell) return;
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

  const choose = (s: Suggestion) => {
    void (s.action ? handleAction(s.action) : handleSuggestionClick(s));
  };

  return { suggestions, choose };
}
