import { useEffect, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ask } from "@tauri-apps/plugin-dialog";
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
  saveNotebook,
  saveNotebookAs,
  openNotebook,
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
  const newNotebook = useNotebookStore((s) => s.newNotebook);
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
        if (cfg.autocomplete_mode) {
          useNotebookStore.getState().setAutocompleteMode(
            cfg.autocomplete_mode as "hint" | "snippet" | "active-hint"
          );
        }
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

  // --- File operations ---

  const handleSave = useCallback(async () => {
    const { cells, filePath, markClean, setFilePath } =
      useNotebookStore.getState();
    const savedPath = await saveNotebook(cells, filePath);
    if (savedPath) {
      setFilePath(savedPath);
      markClean();
    }
  }, []);

  const handleSaveAs = useCallback(async () => {
    const { cells, filePath, markClean, setFilePath } =
      useNotebookStore.getState();
    const savedPath = await saveNotebookAs(cells, filePath);
    if (savedPath) {
      setFilePath(savedPath);
      markClean();
    }
  }, []);

  const handleOpen = useCallback(async () => {
    const { isDirty } = useNotebookStore.getState();
    if (isDirty) {
      const confirmed = await ask("You have unsaved changes. Discard them?", {
        title: "Unsaved Changes",
        kind: "warning",
      });
      if (!confirmed) return;
    }
    const result = await openNotebook();
    if (result) {
      loadNotebook(result.notebook.cells, result.path);
    }
  }, [loadNotebook]);

  const handleNew = useCallback(async () => {
    const { isDirty } = useNotebookStore.getState();
    if (isDirty) {
      const confirmed = await ask("You have unsaved changes. Discard them?", {
        title: "Unsaved Changes",
        kind: "warning",
      });
      if (!confirmed) return;
    }
    newNotebook();
  }, [newNotebook]);

  // --- Listen for native menu events from Tauri ---

  useEffect(() => {
    const unlisten = listen<string>("menu-event", (event) => {
      switch (event.payload) {
        case "new":
          handleNew();
          break;
        case "open":
          handleOpen();
          break;
        case "save":
          handleSave();
          break;
        case "save_as":
          handleSaveAs();
          break;
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [handleNew, handleOpen, handleSave, handleSaveAs]);

  // --- Warn on close with unsaved changes ---

  useEffect(() => {
    const unlisten = getCurrentWindow().onCloseRequested(async (event) => {
      const { isDirty } = useNotebookStore.getState();
      if (isDirty) {
        event.preventDefault();
        const confirmed = await ask(
          "You have unsaved changes. Close without saving?",
          { title: "Unsaved Changes", kind: "warning" }
        );
        if (confirmed) {
          getCurrentWindow().destroy();
        }
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // --- Keyboard shortcut: Cmd+K for command palette ---

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
