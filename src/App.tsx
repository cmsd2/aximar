import { useEffect, useState, useCallback } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { message } from "@tauri-apps/plugin-dialog";
import { t } from "@lingui/core/macro";
import { confirmClose, saveNotebookTab } from "./lib/unsaved-changes";
import { TabBar } from "./components/TabBar";
import { Toolbar } from "./components/Toolbar";
import { VariablePanel } from "./components/VariablePanel";
import { Notebook } from "./components/Notebook";
import { CommandPalette } from "./components/CommandPalette";
import { TemplateChooser } from "./components/TemplateChooser";
import { SettingsModal } from "./components/SettingsModal";
import { ExportLatexModal } from "./components/ExportLatexModal";
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
  nbLoadTemplate,
} from "./lib/notebook-commands";
import {
  getHasSeenWelcome,
  setHasSeenWelcome,
  saveNotebookAs,
  openFile,
  savePlotFile,
} from "./lib/notebooks-client";
import { getConfig } from "./lib/config-client";
import { applyConfig } from "./lib/apply-config";
import { activateLanguage } from "./lib/i18n";
import { TRANSLATOR_TOOLS, applyPreview, type PreviewCatalog } from "./lib/translation-preview";
import { useNotebookStore, useActiveAnyTab, getActiveTabState } from "./store/notebookStore";
import { mapSyncCells, openNotebookTab } from "./hooks/useNotebookEvents";
import { PlotViewer } from "./components/PlotViewer";
import { useLogStore } from "./store/logStore";
import { useSettingsStore } from "./store/settingsStore";
import type { LogLink } from "./types/log";
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
  const settingsOpen = useSettingsStore((s) => s.open);
  const settingsSection = useSettingsStore((s) => s.section);
  const settingsRequest = useSettingsStore((s) => s.request);
  const openSettings = useSettingsStore((s) => s.openSettings);
  const closeSettings = useSettingsStore((s) => s.closeSettings);
  const [variablesOpen, setVariablesOpen] = useState(false);
  const [exportLatexOpen, setExportLatexOpen] = useState(false);
  const [docsOpen, setDocsOpen] = useState(false);
  const [docsFunctionName, setDocsFunctionName] = useState<string | undefined>(undefined);
  const [docsRequestId, setDocsRequestId] = useState(0);
  const addTab = useNotebookStore((s) => s.addTab);
  const setActiveTab = useNotebookStore((s) => s.setActiveTab);
  const windowOpen = useLogStore((s) => s.windowOpen);
  const toggleWindow = useLogStore((s) => s.toggleWindow);
  const logUnreadCount = useLogStore((s) => s.unreadCount);
  const addLogEntry = useLogStore((s) => s.addEntry);
  const addRawOutput = useLogStore((s) => s.addRawOutput);
  const addEnvelope = useLogStore((s) => s.addEnvelope);

  useEffect(() => {
    initSession();
  }, [initSession]);

  // Load config on mount: apply font size and variables_open default
  useEffect(() => {
    getConfig()
      .then(({ config: cfg, warnings }) => {
        const store = useNotebookStore.getState();
        applyConfig(cfg, cfg, {
          setTheme: store.setTheme,
          setCellStyle: store.setCellStyle,
          setAutocompleteMode: store.setAutocompleteMode,
          setVariablesOpen,
        });
        void activateLanguage(cfg.language);
        for (const w of warnings) {
          addLogEntry("warning", w, "config");
        }
      })
      .catch((e) => addLogEntry("error", `Failed to load config: ${e}`, "config"));
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // A translation file being previewed was loaded or saved (pre-release builds)
  useEffect(() => {
    if (!TRANSLATOR_TOOLS) return;
    const unlisten = listen<PreviewCatalog>("translation-preview", (event) => {
      void applyPreview(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // The language setting changed, in this window or another
  useEffect(() => {
    const unlisten = listen<string>("language-changed", (event) => {
      void activateLanguage(event.payload);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

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

  // Listen for kernel-events envelope frames from the backend
  useEffect(() => {
    const unlisten = listen<{
      notebook_id?: string;
      timestamp_ms: number;
      raw_line: string;
      kind: string | null;
      parse_error: string | null;
    }>("maxima-event", (event) => {
      addEnvelope(
        event.payload.timestamp_ms,
        event.payload.raw_line,
        event.payload.kind,
        event.payload.parse_error,
      );
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Listen for structured app log events from the backend, and replay
  // any that were buffered before this listener was ready.
  useEffect(() => {
    type AppLog = { level: string; message: string; source: string; link?: LogLink };
    const unlisten = listen<AppLog>("app-log", (event) => {
      addLogEntry(
        event.payload.level as "info" | "warning" | "error",
        event.payload.message,
        event.payload.source,
        event.payload.link
      );
    });
    // Drain buffered logs that arrived before the listener was mounted
    invoke<AppLog[]>("get_buffered_logs").then((entries) => {
      for (const e of entries) {
        addLogEntry(e.level as "info" | "warning" | "error", e.message, e.source, e.link);
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
          // Load the welcome notebook into the first notebook
          const activeId = useNotebookStore.getState().activeNotebookId;
          await nbLoadTemplate("welcome", activeId ?? undefined);
          // Re-sync the frontend store with the post-load backend
          // state.  useNotebookEvents runs nbList → addTab →
          // nbGetState concurrently with this effect; if its
          // applyBackendState lands after the load's events, the store
          // ends up with the pre-load empty cell instead of the welcome
          // cells.  Refetching here closes that race window.
          if (activeId) {
            try {
              const state = await nbGetState(activeId);
              useNotebookStore.getState().applyBackendState(
                state.notebook_id,
                mapSyncCells(state.cells),
                state.effect,
                state.cell_id ?? undefined,
                state.can_undo,
                state.can_redo,
                state.trusted,
              );
            } catch (e) {
              addLogEntry("error", `Failed to re-sync welcome notebook: ${e}`, "init");
            }
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
    const state = useNotebookStore.getState();
    const id = state.activeNotebookId;
    if (!id) return;
    const activeTab = state.notebooks[id];
    if (!activeTab) return;

    if (activeTab.type === "plot") {
      await savePlotFile(activeTab.plotData, activeTab.filePath);
    } else {
      await saveNotebookTab(id);
    }
  }, []);

  const handleSaveAs = useCallback(async () => {
    const state = useNotebookStore.getState();
    const id = state.activeNotebookId;
    if (!id) return;
    const activeTab = state.notebooks[id];
    if (!activeTab) return;

    if (activeTab.type === "plot") {
      await savePlotFile(activeTab.plotData, null);
    } else {
      // The path_changed event sets the tab's title and save path
      const savedPath = await saveNotebookAs(id, activeTab.filePath);
      if (savedPath) {
        useNotebookStore.getState().markNotebookClean(id);
      }
    }
  }, []);

  const addPlotTab = useNotebookStore((s) => s.addPlotTab);

  const handleOpen = useCallback(async () => {
    const result = await openFile();
    if (!result) return;

    if (result.type === "plot") {
      // Open Plotly JSON as a plot viewer tab
      const title =
        result.path.split("/").pop()?.split("\\").pop() ??
        t({ message: "Plot", comment: "Tab title for an opened plot file whose file name could not be read" });
      const id = `plot-${crypto.randomUUID()}`;
      addPlotTab(id, title, result.plotData, result.path);
    } else {
      try {
        await openNotebookTab(result.path);
      } catch (e) {
        await message(String(e), {
          title: t({
            message: "Couldn't Open Notebook",
            comment: "Title of the error dialog when a notebook file fails to open; the dialog shows the reason below",
          }),
          kind: "error",
        });
      }
    }
  }, [addPlotTab]);

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
        case "export_latex":
          setExportLatexOpen(true);
          break;
        case "settings":
          openSettings();
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
        // One Save, Don't Save or Cancel for every notebook with changes
        if (await confirmClose(Object.keys(notebooks))) {
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

  const activeAnyTab = useActiveAnyTab();
  const isNotebookTab = !activeAnyTab || activeAnyTab.type === "notebook";

  return (
    <div className="app">
      <TabBar />
      {isNotebookTab && (
        <Toolbar
          onOpenTemplates={() => setTemplateChooserOpen(true)}
          onOpenSettings={() => openSettings()}
          variablesOpen={variablesOpen}
          onToggleVariables={() => setVariablesOpen((o) => !o)}
          logOpen={windowOpen}
          onToggleLog={toggleWindow}
          logUnreadCount={logUnreadCount}
          docsOpen={docsOpen}
          onToggleDocs={() => setDocsOpen((o) => !o)}
        />
      )}
      {isNotebookTab && <FindBar />}
      {isNotebookTab && <VariablePanel open={variablesOpen} />}
      <div className="main-content">
        {isNotebookTab && <Notebook onViewDocs={openDocsFor} />}
        {activeAnyTab?.type === "plot" && (
          <PlotViewer plotData={activeAnyTab.plotData} />
        )}
        {isNotebookTab && (
          <DocsPanel
            open={docsOpen}
            functionName={docsFunctionName}
            requestId={docsRequestId}
            onClose={() => setDocsOpen(false)}
          />
        )}
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
        <SettingsModal
          onClose={closeSettings}
          onSetVariablesOpen={setVariablesOpen}
          initialSection={settingsSection ?? undefined}
          request={settingsRequest}
        />
      )}
      {exportLatexOpen && (
        <ExportLatexModal onClose={() => setExportLatexOpen(false)} />
      )}
    </div>
  );
}

export default App;
