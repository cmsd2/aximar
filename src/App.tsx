import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { confirmClose } from "./lib/unsaved-changes";
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
import { ShortcutsDialog } from "./components/ShortcutsDialog";
import { useMaxima } from "./hooks/useMaxima";
import { useNotebookEvents } from "./hooks/useNotebookEvents";
import { useTheme } from "./hooks/useTheme";
import {
  nbGetState,
  nbLoadTemplate,
} from "./lib/notebook-commands";
import {
  getHasSeenWelcome,
  setHasSeenWelcome,
} from "./lib/notebooks-client";
import { getConfig } from "./lib/config-client";
import { applyConfig } from "./lib/apply-config";
import { activateLanguage } from "./lib/i18n";
import { applyPreview, setPreviewError, type PreviewCatalog } from "./lib/translation-preview";
import { useNotebookStore, useActiveAnyTab } from "./store/notebookStore";
import { mapSyncCells } from "./hooks/useNotebookEvents";
import { PlotViewer } from "./components/PlotViewer";
import { DropOverlay } from "./components/DropOverlay";
import { useFileDrop } from "./hooks/useFileDrop";
import { useLogStore } from "./store/logStore";
import { useSettingsStore } from "./store/settingsStore";
import type { LogLink } from "./types/log";
import { useUiStore } from "./store/uiStore";
import { useMenuBar } from "./commands/useMenuBar";
import { drawsWindowFrame } from "./lib/window-frame";
import { TitleBar } from "./components/TitleBar";
import { MenuBar } from "./components/MenuBar";
import "./styles/global.css";

function App() {
  // Notebook files dragged onto the window open in tabs
  const drag = useFileDrop();
  const { initSession } = useMaxima();
  // The menu bar, built from the command list, and its commands
  useMenuBar();
  useTheme();
  useNotebookEvents();
  const paletteOpen = useUiStore((s) => s.paletteOpen);
  const paletteQuery = useUiStore((s) => s.paletteQuery);
  const setPaletteOpen = useUiStore((s) => s.setPaletteOpen);
  const templateChooserOpen = useUiStore((s) => s.templateChooserOpen);
  const setTemplateChooserOpen = useUiStore((s) => s.setTemplateChooserOpen);
  const settingsOpen = useSettingsStore((s) => s.open);
  const settingsSection = useSettingsStore((s) => s.section);
  const settingsRequest = useSettingsStore((s) => s.request);
  const closeSettings = useSettingsStore((s) => s.closeSettings);
  const variablesOpen = useUiStore((s) => s.variablesOpen);
  const setVariablesOpen = useUiStore((s) => s.setVariablesOpen);
  const exportLatexOpen = useUiStore((s) => s.exportLatexOpen);
  const setExportLatexOpen = useUiStore((s) => s.setExportLatexOpen);
  const docsOpen = useUiStore((s) => s.docsOpen);
  const setDocsOpen = useUiStore((s) => s.setDocsOpen);
  const docsFunctionName = useUiStore((s) => s.docsFunctionName);
  const docsRequestId = useUiStore((s) => s.docsRequestId);
  const openDocsFor = useUiStore((s) => s.openDocsFor);
  const shortcutsOpen = useUiStore((s) => s.shortcutsOpen);
  const setShortcutsOpen = useUiStore((s) => s.setShortcutsOpen);
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
          setShowSuggestions: store.setShowSuggestions,
          setVariablesOpen,
        });
        void activateLanguage(cfg.language);
        for (const w of warnings) {
          addLogEntry("warning", w, "config");
        }
      })
      .catch((e) => addLogEntry("error", `Failed to load config: ${e}`, "config"));
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // A translation file being previewed was loaded or saved
  useEffect(() => {
    const unlisten = listen<PreviewCatalog>("translation-preview", (event) => {
      setPreviewError(null);
      void applyPreview(event.payload);
    });
    // A save that doesn't parse: the window keeps the last version that
    // did, and says why in Settings and the log
    const unlistenError = listen<string>("translation-preview-error", (event) => {
      setPreviewError(event.payload);
      addLogEntry("warning", event.payload, "translation");
    });
    return () => {
      unlisten.then((fn) => fn());
      unlistenError.then((fn) => fn());
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

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

  // Keyboard shortcuts are the menu bar's accelerators (src/commands/registry.ts)

  const activeAnyTab = useActiveAnyTab();
  const isNotebookTab = !activeAnyTab || activeAnyTab.type === "notebook";
  // On Windows the page draws the title bar, with the tabs, and the menu bar
  const frame = drawsWindowFrame();

  return (
    <div className="app">
      {frame ? <TitleBar /> : <TabBar />}
      {frame ? (
        <div className="command-bar">
          <MenuBar />
          {isNotebookTab && <Toolbar />}
        </div>
      ) : (
        isNotebookTab && <Toolbar />
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
      <DropOverlay drag={drag} />
      {paletteOpen && (
        <CommandPalette
          onClose={() => setPaletteOpen(false)}
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
      {shortcutsOpen && <ShortcutsDialog onClose={() => setShortcutsOpen(false)} />}
    </div>
  );
}

export default App;
