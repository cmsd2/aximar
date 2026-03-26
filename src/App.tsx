import { useEffect, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { ask } from "@tauri-apps/plugin-dialog";
import { TabBar } from "./components/TabBar";
import { Toolbar } from "./components/Toolbar";
import { VariablePanel } from "./components/VariablePanel";
import { Notebook } from "./components/Notebook";
import { CommandPalette } from "./components/CommandPalette";
import { TemplateChooser } from "./components/TemplateChooser";
import { SettingsModal } from "./components/SettingsModal";
import { StatusBar, LogWindow } from "./components/LogPanel";
import { DocsPanel } from "./components/DocsPanel";
import { FindBar } from "./components/FindBar";
import { ShortcutHints } from "./components/ShortcutHints";
import { useMaxima } from "./hooks/useMaxima";
import { useNotebookEvents } from "./hooks/useNotebookEvents";
import { useTheme } from "./hooks/useTheme";
import {
  nbDeleteCell,
  nbMoveCell,
  nbUndo,
  nbRedo,
  nbCreate,
  nbSetActive,
  nbGetState,
  nbLoadCells,
} from "./lib/notebook-commands";
import {
  getHasSeenWelcome,
  setHasSeenWelcome,
  getTemplate,
  saveNotebook,
  saveNotebookAs,
  openNotebook,
} from "./lib/notebooks-client";
import { getConfig, markdownFontStack, applyPrintMargins } from "./lib/config-client";
import { useNotebookStore, getActiveTabState } from "./store/notebookStore";
import { useLogStore } from "./store/logStore";
import { useFindStore } from "./store/findStore";
import "./styles/global.css";

function App() {
  const { initSession } = useMaxima();
  useTheme();
  useNotebookEvents();
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
  const setFilePath = useNotebookStore((s) => s.setFilePath);
  const markClean = useNotebookStore((s) => s.markClean);
  const windowOpen = useLogStore((s) => s.windowOpen);
  const toggleWindow = useLogStore((s) => s.toggleWindow);
  const logUnreadCount = useLogStore((s) => s.unreadCount);
  const addLogEntry = useLogStore((s) => s.addEntry);
  const addRawOutput = useLogStore((s) => s.addRawOutput);

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
        document.documentElement.style.setProperty(
          "--print-font-size",
          `${cfg.print_font_size}px`
        );
        document.documentElement.style.setProperty(
          "--print-font-size-mono",
          `${cfg.print_font_size - 1}px`
        );
        document.documentElement.dataset.cellStyle = cfg.cell_style || "bracket";
        document.documentElement.style.setProperty(
          "--font-family-markdown",
          markdownFontStack(cfg.markdown_font)
        );
        document.documentElement.style.setProperty(
          "--markdown-indent",
          cfg.markdown_indent === "aligned" ? "var(--gutter-width)" : "16px"
        );
        applyPrintMargins(
          cfg.print_margin_top, cfg.print_margin_bottom,
          cfg.print_margin_left, cfg.print_margin_right
        );
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
      .catch((e) => addLogEntry("error", `Failed to load config: ${e}`, "config"));
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Listen for raw Maxima output events from the backend
  useEffect(() => {
    const unlisten = listen<{ notebook_id?: string; line: string; stream: string; timestamp: number }>(
      "maxima-output",
      (event) => {
        addRawOutput(
          event.payload.line,
          event.payload.stream as "stdin" | "stdout" | "stderr",
          event.payload.timestamp
        );
      }
    );
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Listen for structured app log events from the backend, and replay
  // any that were buffered before this listener was ready.
  useEffect(() => {
    const unlisten = listen<{ level: string; message: string; source: string }>(
      "app-log",
      (event) => {
        addLogEntry(
          event.payload.level as "info" | "warning" | "error",
          event.payload.message,
          event.payload.source
        );
      }
    );
    // Drain buffered logs that arrived before the listener was mounted
    invoke<{ level: string; message: string; source: string }[]>(
      "get_buffered_logs"
    ).then((entries) => {
      for (const e of entries) {
        addLogEntry(
          e.level as "info" | "warning" | "error",
          e.message,
          e.source
        );
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // First-launch: load welcome notebook
  useEffect(() => {
    getHasSeenWelcome()
      .then(async (seen) => {
        if (!seen) {
          const welcome = await getTemplate("welcome");
          if (welcome) {
            const cells = welcome.cells
              .filter((c) => c.cell_type !== "raw")
              .map((c) => ({
                id: crypto.randomUUID(),
                cell_type: c.cell_type === "markdown" ? "markdown" : "code",
                input: typeof c.source === "string" ? c.source : (c.source as string[]).join(""),
              }));
            await nbLoadCells(cells);
          }
          await setHasSeenWelcome();
        }
      })
      .catch((e) => addLogEntry("error", `Failed to load welcome notebook: ${e}`, "init"));
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const openDocsFor = useCallback((name: string) => {
    setDocsFunctionName(name);
    setDocsRequestId((n) => n + 1);
    setDocsOpen(true);
  }, []);

  // --- File operations ---

  const handleSave = useCallback(async () => {
    const { markClean, setFilePath } = useNotebookStore.getState();
    const tab = getActiveTabState();
    const savedPath = await saveNotebook(tab.cells, tab.filePath);
    if (savedPath) {
      setFilePath(savedPath);
      markClean();
    }
  }, []);

  const handleSaveAs = useCallback(async () => {
    const { markClean, setFilePath } = useNotebookStore.getState();
    const tab = getActiveTabState();
    const savedPath = await saveNotebookAs(tab.cells, tab.filePath);
    if (savedPath) {
      setFilePath(savedPath);
      markClean();
    }
  }, []);

  const handleOpen = useCallback(async () => {
    const tab = getActiveTabState();
    if (tab.isDirty) {
      const confirmed = await ask("You have unsaved changes. Discard them?", {
        title: "Unsaved Changes",
        kind: "warning",
      });
      if (!confirmed) return;
    }
    const result = await openNotebook();
    if (result) {
      const cells = result.notebook.cells
        .filter((c) => c.cell_type !== "raw")
        .map((c) => ({
          id: crypto.randomUUID(),
          cell_type: c.cell_type === "markdown" ? "markdown" : "code",
          input: typeof c.source === "string" ? c.source : (c.source as string[]).join(""),
        }));
      await nbLoadCells(cells);
      setFilePath(result.path);
      markClean();
    }
  }, [setFilePath, markClean]);

  const addTab = useNotebookStore((s) => s.addTab);
  const setActiveTab = useNotebookStore((s) => s.setActiveTab);

  const handleNew = useCallback(async () => {
    const result = await nbCreate();
    addTab(result.notebook_id);
    setActiveTab(result.notebook_id);
    await nbSetActive(result.notebook_id);
    const state = await nbGetState(result.notebook_id);
    useNotebookStore.getState().applyBackendState(
      state.notebook_id,
      state.cells.map((sc) => ({
        id: sc.id,
        cellType: sc.cell_type as "code" | "markdown",
        input: sc.input,
        output: null,
        status: "idle" as const,
      })),
      state.effect,
      state.cell_id ?? undefined,
      state.can_undo,
      state.can_redo,
    );
  }, [addTab, setActiveTab]);

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
      // Check if any notebook has unsaved changes
      const { notebooks } = useNotebookStore.getState();
      const hasUnsaved = Object.values(notebooks).some((tab) => tab.isDirty);
      if (hasUnsaved) {
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

  // --- Notebook keyboard shortcuts ---

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const mod = e.metaKey || e.ctrlKey;
      if (!mod) return;

      // Skip when focus is inside a modal
      const target = e.target as Element | null;
      if (target?.closest(".palette-overlay, .settings-modal, .template-modal")) {
        return;
      }

      const key = e.key.toLowerCase();

      if (key === "z" && !e.shiftKey) {
        e.preventDefault();
        nbUndo();
      } else if (key === "z" && e.shiftKey) {
        e.preventDefault();
        nbRedo();
      } else if (key === "y") {
        e.preventDefault();
        nbRedo();
      } else if (key === "f" && !e.shiftKey) {
        e.preventDefault();
        useFindStore.getState().open(false);
      } else if (key === "f" && e.shiftKey) {
        e.preventDefault();
        useFindStore.getState().open(true);
      } else if (key === "d") {
        e.preventDefault();
        const tab = getActiveTabState();
        if (tab.activeCellId && tab.cells.length > 1) {
          nbDeleteCell(tab.activeCellId);
        }
      } else if (e.shiftKey && key === "arrowup") {
        e.preventDefault();
        const tab = getActiveTabState();
        if (tab.activeCellId) nbMoveCell(tab.activeCellId, "up");
      } else if (e.shiftKey && key === "arrowdown") {
        e.preventDefault();
        const tab = getActiveTabState();
        if (tab.activeCellId) nbMoveCell(tab.activeCellId, "down");
      }
    };

    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  return (
    <div className="app">
      <TabBar />
      <Toolbar
        onOpenTemplates={() => setTemplateChooserOpen(true)}
        onOpenSettings={() => setSettingsOpen(true)}
        variablesOpen={variablesOpen}
        onToggleVariables={() => setVariablesOpen((o) => !o)}
        logOpen={windowOpen}
        onToggleLog={toggleWindow}
        logUnreadCount={logUnreadCount}
        docsOpen={docsOpen}
        onToggleDocs={() => setDocsOpen((o) => !o)}
      />
      <FindBar />
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
      <LogWindow />
      <StatusBar />
      <ShortcutHints />
      {paletteOpen && (
        <CommandPalette
          onClose={() => {
            setPaletteOpen(false);
            setPaletteQuery(undefined);
          }}
          onViewDocs={openDocsFor}
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
