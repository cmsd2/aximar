import { useState, useEffect, useRef } from "react";
import { useNotebookStore } from "../store/notebookStore";
import { useMaxima } from "../hooks/useMaxima";
import { getSuggestions } from "../lib/suggestions-client";
import type { Suggestion } from "../types/suggestions";
import type { Cell } from "../types/notebook";
import type { EvalResult } from "../types/maxima";

/**
 * Replace standalone `%` with a specific Maxima output label.
 * Leaves `%pi`, `%e`, `%i`, `%oN`, `%iN` etc. untouched.
 */
function bindPercent(expr: string, label: string): string {
  return expr.replace(/(?<![a-zA-Z0-9_])%(?![a-zA-Z0-9_])/g, label);
}

interface CellSuggestionsProps {
  cell: Cell;
}

export function CellSuggestions({ cell }: CellSuggestionsProps) {
  const [suggestions, setSuggestions] = useState<Suggestion[]>([]);
  const addCellWithInput = useNotebookStore((s) => s.addCellWithInput);
  const updateCellInput = useNotebookStore((s) => s.updateCellInput);
  const cells = useNotebookStore((s) => s.cells);
  const { executeCell } = useMaxima();

  // Track the cell created by the last suggestion click so we can reuse it
  const lastSuggestionRef = useRef<{ cellId: string; template: string } | null>(null);

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

  // Real Maxima label for this cell's output (e.g. "%o6")
  const realLabel = cell.output?.outputLabel;

  const handleSuggestionClick = (template: string) => {
    const prev = lastSuggestionRef.current;

    // Rewrite bare % → real Maxima %oN for stable evaluation
    const evalExpr = realLabel ? bindPercent(template, realLabel) : template;

    // Check if we can reuse the cell from the previous suggestion click:
    // it must still exist, and its input must not have been edited by the user
    if (prev) {
      const prevCell = cells.find((c) => c.id === prev.cellId);
      if (prevCell && prevCell.input === prev.template) {
        // Reuse: update input and re-execute
        updateCellInput(prev.cellId, template);
        lastSuggestionRef.current = { cellId: prev.cellId, template };
        executeCell(prev.cellId, evalExpr);
        return;
      }
    }

    // Create a new cell (shows clean % in input)
    const newCellId = addCellWithInput(cell.id, template);
    lastSuggestionRef.current = { cellId: newCellId, template };
    executeCell(newCellId, evalExpr);
  };

  return (
    <div className="cell-suggestions">
      {suggestions.map((s) => (
        <button
          key={s.template}
          className="suggestion-chip"
          title={s.description}
          onClick={() => handleSuggestionClick(s.template)}
        >
          {s.label}
        </button>
      ))}
    </div>
  );
}
