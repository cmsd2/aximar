import { useState, useEffect, useCallback } from "react";
import { plural } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { listTemplates } from "../lib/notebooks-client";
import { nbCreate, nbSetActive, nbLoadTemplate } from "../lib/notebook-commands";
import { useNotebookStore } from "../store/notebookStore";
import type { TemplateSummary } from "../types/notebooks";

interface TemplateChooserProps {
  onClose: () => void;
}

export function TemplateChooser({ onClose }: TemplateChooserProps) {
  const { t } = useLingui();
  const [templates, setTemplates] = useState<TemplateSummary[]>([]);

  useEffect(() => {
    listTemplates()
      .then(setTemplates)
      .catch(() => {});
  }, []);

  const handleSelect = useCallback(
    async (id: string) => {
      // Create a new notebook and load the template into it
      const result = await nbCreate();
      const nbId = result.notebook_id;
      const store = useNotebookStore.getState();
      store.addTab(nbId);
      store.setActiveTab(nbId);
      await nbSetActive(nbId);
      await nbLoadTemplate(id, nbId);
      onClose();
    },
    [onClose]
  );

  return (
    <div className="palette-overlay" onClick={onClose}>
      <div
        className="template-modal"
        onClick={(e) => e.stopPropagation()}
      >
        <div className="template-header">
          <h2 className="template-title">
            {t({ message: "Starter Notebooks", comment: "Template chooser: the title of the window that offers example notebooks" })}
          </h2>
          <p className="template-subtitle">
            {t({
              message: "Choose a template to get started with Maxima",
              comment: "Template chooser: the line under the title",
            })}
          </p>
        </div>
        <div className="template-list">
          {templates.map((tpl) => {
            const count = tpl.cell_count;
            return (
              <button
                key={tpl.id}
                className="template-card"
                onClick={() => handleSelect(tpl.id)}
              >
                <div className="template-card-title">{tpl.title}</div>
                <div className="template-card-desc">{tpl.description}</div>
                <div className="template-card-meta">
                  {t({
                    message: plural(count, { one: "# cell", other: "# cells" }),
                    comment: "Template chooser: the number of cells in a starter notebook, under its description",
                  })}
                </div>
              </button>
            );
          })}
        </div>
        <div className="template-footer">
          <button className="template-skip" onClick={onClose}>
            {t({
              message: "Start with empty notebook",
              comment: "Template chooser button: closes the chooser and keeps the empty notebook",
            })}
          </button>
        </div>
      </div>
    </div>
  );
}
