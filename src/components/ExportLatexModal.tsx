import { useState, useCallback } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { useActiveTab, useNotebookStore } from "../store/notebookStore";
import { exportCellsAsLatex, type ExportOptions } from "../lib/latex-export";

interface ExportLatexModalProps {
  onClose: () => void;
}

export function ExportLatexModal({ onClose }: ExportLatexModalProps) {
  const tab = useActiveTab();
  const clearSelection = useNotebookStore((s) => s.clearSelection);
  const [includeCode, setIncludeCode] = useState(true);
  const [includePlots, setIncludePlots] = useState(true);
  const [exporting, setExporting] = useState(false);

  const selectedIds = tab.selectedCellIds;
  const hasSelection = selectedIds.length > 0;
  const cellsToExport = hasSelection
    ? tab.cells.filter((c) => selectedIds.includes(c.id))
    : tab.cells;

  const handleExport = useCallback(async () => {
    const path = await save({
      defaultPath: (tab.title.replace(/\.\w+$/, "") || "notebook") + ".tex",
      filters: [{ name: "LaTeX Document", extensions: ["tex"] }],
    });
    if (!path) return;

    setExporting(true);
    try {
      const options: ExportOptions = { includeCode, includePlots };
      const result = await exportCellsAsLatex(cellsToExport, options);

      // Create images directory if needed
      if (result.images.size > 0) {
        const dir = path.replace(/[/\\][^/\\]+$/, "") + "/images";
        await invoke("ensure_directory", { path: dir });

        // Write each image file
        const basePath = path.replace(/[/\\][^/\\]+$/, "");
        for (const [filename, data] of result.images) {
          const imgPath = basePath + "/" + filename;
          await invoke("write_binary_file", {
            path: imgPath,
            data: Array.from(data),
          });
        }
      }

      // Write the .tex file
      await invoke("write_text_file", { path, content: result.tex });
      clearSelection();
      onClose();
    } catch (err) {
      console.error("LaTeX export failed:", err);
    } finally {
      setExporting(false);
    }
  }, [tab.title, includeCode, includePlots, cellsToExport, clearSelection, onClose]);

  return (
    <div className="palette-overlay" onClick={onClose}>
      <div className="settings-modal export-latex-modal" onClick={(e) => e.stopPropagation()}>
        <div className="settings-header">
          <h2 className="settings-title">Export as LaTeX</h2>
        </div>
        <div className="settings-body">
          <div className="export-latex-scope">
            {hasSelection
              ? `Exporting ${cellsToExport.length} selected cell${cellsToExport.length === 1 ? "" : "s"}`
              : `Exporting entire notebook (${cellsToExport.length} cell${cellsToExport.length === 1 ? "" : "s"})`}
          </div>

          <div className="settings-section">
            <div className="settings-row">
              <label className="settings-label">Include code input</label>
              <div className="settings-control">
                <input
                  type="checkbox"
                  className="settings-checkbox"
                  checked={includeCode}
                  onChange={(e) => setIncludeCode(e.target.checked)}
                />
              </div>
            </div>
            <div className="settings-row">
              <label className="settings-label">Include plots</label>
              <div className="settings-control">
                <input
                  type="checkbox"
                  className="settings-checkbox"
                  checked={includePlots}
                  onChange={(e) => setIncludePlots(e.target.checked)}
                />
              </div>
            </div>
          </div>
        </div>
        <div className="settings-footer">
          <button className="template-skip" onClick={onClose}>
            Cancel
          </button>
          <button
            className="template-start"
            onClick={handleExport}
            disabled={exporting}
          >
            {exporting ? "Exporting..." : "Export"}
          </button>
        </div>
      </div>
    </div>
  );
}
