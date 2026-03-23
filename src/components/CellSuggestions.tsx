import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { useNotebookStore } from "../store/notebookStore";
import { useMaxima } from "../hooks/useMaxima";
import { getSuggestions } from "../lib/suggestions-client";
import type { Suggestion } from "../types/suggestions";
import type { Cell } from "../types/notebook";
import type { EvalResult } from "../types/maxima";

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
      }
    },
    [cell.output],
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

  const handleSuggestionClick = (template: string) => {
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

    // Create a new cell — backend resolves % via label context
    const newCellId = addCellWithInput(cell.id, template);
    lastSuggestionRef.current = { cellId: newCellId, template };
    executeCell(newCellId, template);
  };

  return (
    <div className="cell-suggestions">
      {suggestions.map((s) => (
        <button
          key={s.action ?? s.template}
          className="suggestion-chip"
          title={s.description}
          onClick={() =>
            s.action ? handleAction(s.action) : handleSuggestionClick(s.template)
          }
        >
          {s.label}
        </button>
      ))}
    </div>
  );
}
