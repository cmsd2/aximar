import { useState, useEffect, useCallback } from "react";
import { listTemplates } from "../lib/notebooks-client";
import { nbCreate, nbSetActive, nbLoadTemplate } from "../lib/notebook-commands";
import { useNotebookStore } from "../store/notebookStore";
import type { TemplateSummary } from "../types/notebooks";

interface TemplateChooserProps {
  onClose: () => void;
}

export function TemplateChooser({ onClose }: TemplateChooserProps) {
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
          <h2 className="template-title">Starter Notebooks</h2>
          <p className="template-subtitle">
            Choose a template to get started with Maxima
          </p>
        </div>
        <div className="template-list">
          {templates.map((t) => (
            <button
              key={t.id}
              className="template-card"
              onClick={() => handleSelect(t.id)}
            >
              <div className="template-card-title">{t.title}</div>
              <div className="template-card-desc">{t.description}</div>
              <div className="template-card-meta">
                {t.cell_count} cells
              </div>
            </button>
          ))}
        </div>
        <div className="template-footer">
          <button className="template-skip" onClick={onClose}>
            Start with empty notebook
          </button>
        </div>
      </div>
    </div>
  );
}
