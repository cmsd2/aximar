import { useCallback, useEffect, useState, useRef } from "react";
import { ask } from "@tauri-apps/plugin-dialog";
import { useNotebookStore } from "../store/notebookStore";
import type { NotebookTab } from "../store/notebookStore";
import { nbCreate, nbClose, nbSetActive, nbGetState, createWindow } from "../lib/notebook-commands";

function statusDotClass(status: NotebookTab["sessionStatus"]): string {
  if (status === "Ready") return "tab-status-dot ready";
  if (status === "Starting" || status === "Busy") return "tab-status-dot busy";
  if (status === "Stopped") return "tab-status-dot stopped";
  return "tab-status-dot error";
}

interface ContextMenuState {
  x: number;
  y: number;
  tabId: string;
}

export function TabBar() {
  const notebooks = useNotebookStore((s) => s.notebooks);
  const activeNotebookId = useNotebookStore((s) => s.activeNotebookId);
  const addTab = useNotebookStore((s) => s.addTab);
  const removeTab = useNotebookStore((s) => s.removeTab);
  const setActiveTab = useNotebookStore((s) => s.setActiveTab);
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const contextMenuRef = useRef<HTMLDivElement>(null);

  const tabs = Object.values(notebooks);

  const handleNewTab = useCallback(async () => {
    const result = await nbCreate();
    addTab(result.notebook_id);
    setActiveTab(result.notebook_id);
    await nbSetActive(result.notebook_id);
    // Fetch initial state for the new notebook
    const state = await nbGetState(result.notebook_id);
    useNotebookStore.getState().applyBackendState(
      state.notebook_id,
      state.cells.map((sc) => ({
        id: sc.id,
        cellType: (sc.cell_type as "code" | "markdown"),
        input: sc.input,
        output: null,
        status: "idle" as const,
      })),
      state.effect,
      state.cell_id ?? undefined,
      state.can_undo,
      state.can_redo
    );
  }, [addTab, setActiveTab]);

  const handleCloseTab = useCallback(async (id: string) => {
    if (tabs.length <= 1) return;
    const tab = notebooks[id];
    if (tab?.isDirty) {
      const confirmed = await ask("You have unsaved changes. Close without saving?", {
        title: "Unsaved Changes",
        kind: "warning",
      });
      if (!confirmed) return;
    }
    try {
      await nbClose(id);
      removeTab(id);
    } catch (e) {
      console.warn("Failed to close tab:", e);
    }
  }, [tabs.length, notebooks, removeTab]);

  const handleSwitchTab = useCallback(async (id: string) => {
    setActiveTab(id);
    await nbSetActive(id);
  }, [setActiveTab]);

  const handleMoveToNewWindow = useCallback(async (tabId: string) => {
    setContextMenu(null);
    if (tabs.length <= 1) return; // Don't move the last tab
    await createWindow(tabId);
    removeTab(tabId);
  }, [tabs.length, removeTab]);

  // Dismiss context menu on click outside or Escape
  useEffect(() => {
    if (!contextMenu) return;
    const handleClick = (e: MouseEvent) => {
      if (contextMenuRef.current && !contextMenuRef.current.contains(e.target as Node)) {
        setContextMenu(null);
      }
    };
    const handleKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setContextMenu(null);
    };
    document.addEventListener("mousedown", handleClick);
    document.addEventListener("keydown", handleKey);
    return () => {
      document.removeEventListener("mousedown", handleClick);
      document.removeEventListener("keydown", handleKey);
    };
  }, [contextMenu]);

  // Keyboard shortcuts: Cmd+T (new), Cmd+W (close), Cmd+Shift+[/] / Ctrl+Tab (switch)
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // Skip when focus is inside a modal
      const target = e.target as Element | null;
      if (target?.closest(".palette-overlay, .settings-modal, .template-modal")) {
        return;
      }

      // Ctrl+Tab / Ctrl+Shift+Tab — cycle tabs
      if (e.ctrlKey && e.key === "Tab") {
        e.preventDefault();
        if (activeNotebookId && tabs.length > 1) {
          const idx = tabs.findIndex((t) => t.id === activeNotebookId);
          const next = e.shiftKey
            ? (idx - 1 + tabs.length) % tabs.length
            : (idx + 1) % tabs.length;
          handleSwitchTab(tabs[next].id);
        }
        return;
      }

      const mod = e.metaKey || e.ctrlKey;
      if (!mod) return;

      if (e.key === "t" && !e.shiftKey) {
        e.preventDefault();
        handleNewTab();
      } else if (e.key === "w" && !e.shiftKey) {
        e.preventDefault();
        if (activeNotebookId && tabs.length > 1) {
          handleCloseTab(activeNotebookId);
        }
      } else if (e.shiftKey && e.key === "[") {
        e.preventDefault();
        if (activeNotebookId) {
          const idx = tabs.findIndex((t) => t.id === activeNotebookId);
          if (idx > 0) handleSwitchTab(tabs[idx - 1].id);
        }
      } else if (e.shiftKey && e.key === "]") {
        e.preventDefault();
        if (activeNotebookId) {
          const idx = tabs.findIndex((t) => t.id === activeNotebookId);
          if (idx < tabs.length - 1) handleSwitchTab(tabs[idx + 1].id);
        }
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [activeNotebookId, tabs, handleNewTab, handleCloseTab, handleSwitchTab]);

  return (
    <div className="tab-bar">
      {tabs.map((tab) => (
        <div
          key={tab.id}
          className={`tab ${tab.id === activeNotebookId ? "tab-active" : ""} ${tab.closePending ? "tab-close-pending" : ""}`}
          onClick={() => handleSwitchTab(tab.id)}
          onContextMenu={(e) => {
            e.preventDefault();
            setContextMenu({ x: e.clientX, y: e.clientY, tabId: tab.id });
          }}
        >
          <span className={statusDotClass(tab.sessionStatus)} />
          <span className="tab-title">
            {tab.isDirty ? `${tab.title} *` : tab.title}
          </span>
          {tabs.length > 1 && (
            <button
              className="tab-close"
              onClick={(e) => {
                e.stopPropagation();
                handleCloseTab(tab.id);
              }}
              title="Close tab"
            >
              &times;
            </button>
          )}
        </div>
      ))}
      <button className="tab-new" onClick={handleNewTab} title="New notebook (Cmd+T)">
        +
      </button>
      {contextMenu && (
        <div
          ref={contextMenuRef}
          className="tab-context-menu"
          style={{ left: contextMenu.x, top: contextMenu.y }}
        >
          <button
            className="tab-context-menu-item"
            disabled={tabs.length <= 1}
            onClick={() => handleMoveToNewWindow(contextMenu.tabId)}
          >
            Move to New Window
          </button>
          <button
            className="tab-context-menu-item"
            disabled={tabs.length <= 1}
            onClick={() => {
              setContextMenu(null);
              handleCloseTab(contextMenu.tabId);
            }}
          >
            Close Tab
          </button>
        </div>
      )}
    </div>
  );
}
