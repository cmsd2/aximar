import { useEffect, useState, useCallback } from "react";
import { Toolbar } from "./components/Toolbar";
import { VariablePanel } from "./components/VariablePanel";
import { Notebook } from "./components/Notebook";
import { CommandPalette } from "./components/CommandPalette";
import { TemplateChooser } from "./components/TemplateChooser";
import { SettingsModal } from "./components/SettingsModal";
import { LogPanel } from "./components/LogPanel";
import { DocsPanel } from "./components/DocsPanel";
import { useMaxima } from "./hooks/useMaxima";
import { useTheme } from "./hooks/useTheme";
import {
  getHasSeenWelcome,
  setHasSeenWelcome,
  getTemplate,
} from "./lib/notebooks-client";
import { getConfig } from "./lib/config-client";
import { useNotebookStore } from "./store/notebookStore";
import { useLogStore } from "./store/logStore";
import "./styles/global.css";

function App() {
  const { initSession } = useMaxima();
  useTheme();
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [paletteQuery, setPaletteQuery] = useState<string | undefined>(
    undefined
  );
  const [templateChooserOpen, setTemplateChooserOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [variablesOpen, setVariablesOpen] = useState(false);
  const [docsOpen, setDocsOpen] = useState(false);
  const [docsFunctionName, setDocsFunctionName] = useState<string | undefined>(undefined);
  const [docsRequestId, setDocsRequestId] = useState(0);
  const loadNotebook = useNotebookStore((s) => s.loadNotebook);
  const logOpen = useLogStore((s) => s.logOpen);
  const toggleLog = useLogStore((s) => s.toggleLog);
  const logUnreadCount = useLogStore((s) => s.unreadCount);
  const addLogEntry = useLogStore((s) => s.addEntry);

  useEffect(() => {
    initSession();
  }, [initSession]);

  // Load config on mount: apply font size and variables_open default
  useEffect(() => {
    getConfig()
      .then(({ config: cfg, warnings }) => {
        document.documentElement.style.setProperty(
          "--font-size-mono",
          `${cfg.font_size}px`
        );
        document.documentElement.dataset.cellStyle = cfg.cell_style || "bracket";
        setVariablesOpen(cfg.variables_open);
        for (const w of warnings) {
          addLogEntry("warning", w, "config");
        }
      })
      .catch(() => {});
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

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

  const openDocsFor = useCallback((name: string) => {
    setDocsFunctionName(name);
    setDocsRequestId((n) => n + 1);
    setDocsOpen(true);
  }, []);

  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "k") {
      e.preventDefault();
      setPaletteQuery(undefined);
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
        onOpenSettings={() => setSettingsOpen(true)}
        variablesOpen={variablesOpen}
        onToggleVariables={() => setVariablesOpen((o) => !o)}
        logOpen={logOpen}
        onToggleLog={toggleLog}
        logUnreadCount={logUnreadCount}
        docsOpen={docsOpen}
        onToggleDocs={() => setDocsOpen((o) => !o)}
      />
      <VariablePanel open={variablesOpen} />
      <div className="main-content">
        <Notebook onViewDocs={openDocsFor} />
        <DocsPanel
          open={docsOpen}
          functionName={docsFunctionName}
          requestId={docsRequestId}
          onClose={() => setDocsOpen(false)}
        />
      </div>
      <LogPanel open={logOpen} />
      {paletteOpen && (
        <CommandPalette
          onClose={() => {
            setPaletteOpen(false);
            setPaletteQuery(undefined);
          }}
          initialQuery={paletteQuery}
        />
      )}
      {templateChooserOpen && (
        <TemplateChooser onClose={() => setTemplateChooserOpen(false)} />
      )}
      {settingsOpen && (
        <SettingsModal onClose={() => setSettingsOpen(false)} onSetVariablesOpen={setVariablesOpen} />
      )}
    </div>
  );
}

export default App;
