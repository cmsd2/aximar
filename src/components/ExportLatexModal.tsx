import { useState, useCallback } from "react";
import { save } from "@tauri-apps/plugin-dialog";
import { plural } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { invoke } from "@tauri-apps/api/core";
import { useActiveTab, useNotebookStore } from "../store/notebookStore";
import { exportCellsAsLatex, type ExportOptions } from "../lib/latex-export";
import { Button } from "./ui/Button";
import { Checkbox } from "./ui/Checkbox";
import { Dialog } from "./ui/Dialog";
import { FieldRow } from "./ui/FieldRow";

interface ExportLatexModalProps {
  onClose: () => void;
}

export function ExportLatexModal({ onClose }: ExportLatexModalProps) {
  const { t } = useLingui();
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
    const fallbackName = t({
      message: "notebook",
      comment: "Export as LaTeX: the file name offered in the save dialog when the notebook has no title, before .tex",
    });
    const path = await save({
      defaultPath: (tab.title.replace(/\.\w+$/, "") || fallbackName) + ".tex",
      filters: [
        {
          name: t({ message: "LaTeX Document", comment: "Export as LaTeX: the file type in the save dialog" }),
          extensions: ["tex"],
        },
      ],
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
  }, [t, tab.title, includeCode, includePlots, cellsToExport, clearSelection, onClose]);

  const count = cellsToExport.length;

  return (
    <Dialog
      title={t({ message: "Export as LaTeX", comment: "Export as LaTeX: the title of the export window" })}
      onClose={onClose}
      footer={
        <>
          <Button variant="quiet" onClick={onClose}>
            {t({ message: "Cancel", comment: "Export as LaTeX button: closes the window without exporting" })}
          </Button>
          <Button variant="primary" onClick={handleExport} disabled={exporting}>
            {exporting
              ? t({ message: "Exporting...", comment: "Export as LaTeX button, while the export runs" })
              : t({ message: "Export", comment: "Export as LaTeX button: asks where to save the .tex file and writes it" })}
          </Button>
        </>
      }
    >
      <p className="export-latex-scope">
        {hasSelection
          ? t({
              message: plural(count, {
                one: "Exporting # selected cell",
                other: "Exporting # selected cells",
              }),
              comment: "Export as LaTeX: says how many selected cells the export covers",
            })
          : t({
              message: plural(count, {
                one: "Exporting entire notebook (# cell)",
                other: "Exporting entire notebook (# cells)",
              }),
              comment: "Export as LaTeX: says the export covers the whole notebook, and how many cells it has",
            })}
      </p>
      <FieldRow
        label={t({
          message: "Include code input",
          comment: "Export as LaTeX: checkbox label; when on, the export includes each cell's Maxima input",
        })}
      >
        {({ id }) => <Checkbox id={id} checked={includeCode} onChange={setIncludeCode} />}
      </FieldRow>
      <FieldRow
        label={t({
          message: "Include plots",
          comment: "Export as LaTeX: checkbox label; when on, the export includes plots as image files",
        })}
      >
        {({ id }) => <Checkbox id={id} checked={includePlots} onChange={setIncludePlots} />}
      </FieldRow>
    </Dialog>
  );
}
