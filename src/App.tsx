import { useEffect, useState, useCallback } from "react";
import { Toolbar } from "./components/Toolbar";
import { VariablePanel } from "./components/VariablePanel";
import { Notebook } from "./components/Notebook";
import { CommandPalette } from "./components/CommandPalette";
import { TemplateChooser } from "./components/TemplateChooser";
import { useMaxima } from "./hooks/useMaxima";
import { useTheme } from "./hooks/useTheme";
import {
  getHasSeenWelcome,
  setHasSeenWelcome,
  getTemplate,
} from "./lib/notebooks-client";
import { useNotebookStore } from "./store/notebookStore";
import "./styles/global.css";

function App() {
  const { initSession } = useMaxima();
  useTheme();
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [templateChooserOpen, setTemplateChooserOpen] = useState(false);
  const [variablesOpen, setVariablesOpen] = useState(false);
  const loadNotebook = useNotebookStore((s) => s.loadNotebook);

  useEffect(() => {
    initSession();
  }, [initSession]);

  // First-launch: load welcome notebook
  useEffect(() => {
    getHasSeenWelcome()
      .then(async (seen) => {
        if (!seen) {
          const welcome = await getTemplate("welcome");
          if (welcome) {
            loadNotebook(welcome.cells);
          }
          await setHasSeenWelcome();
        }
      })
      .catch(() => {});
  }, [loadNotebook]);

  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "k") {
      e.preventDefault();
      setPaletteOpen((open) => !open);
    }
  }, []);

  useEffect(() => {
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  return (
    <div className="app">
      <Toolbar
        onOpenTemplates={() => setTemplateChooserOpen(true)}
        variablesOpen={variablesOpen}
        onToggleVariables={() => setVariablesOpen((o) => !o)}
      />
      <VariablePanel open={variablesOpen} />
      <Notebook />
      {paletteOpen && (
        <CommandPalette onClose={() => setPaletteOpen(false)} />
      )}
      {templateChooserOpen && (
        <TemplateChooser onClose={() => setTemplateChooserOpen(false)} />
      )}
    </div>
  );
}

export default App;
