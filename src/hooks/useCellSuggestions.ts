// The suggestions for a code cell's result, such as "Simplify" or "Plot",
// and what choosing one does: insert a cell after it with the
// suggestion's code and run it.
import { useState, useEffect, useRef } from "react";
import { useActiveTab, useNotebookStore } from "../store/notebookStore";
import { useMaxima } from "./useMaxima";
import { getSuggestions } from "../lib/suggestions-client";
import { nbAddCell } from "../lib/notebook-commands";
import type { Suggestion } from "../types/suggestions";
import type { Cell } from "../types/notebook";
import type { EvalResult } from "../types/maxima";

export interface CellSuggestionsState {
  suggestions: Suggestion[];
  /** Insert the suggestion's cell after the result and run it */
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

    // Only the ones that insert a cell: saving a plot is the output's
    // toolbar's job
    getSuggestions(cell.input, output)
      .then((found) => setSuggestions((found ?? []).filter((s) => !s.action)))
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
    void handleSuggestionClick(s);
  };

  return { suggestions, choose };
}
