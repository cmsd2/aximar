import { useCallback, useEffect, useState, useRef } from "react";
import { openNewNotebook } from "../lib/new-notebook";
import { useLingui } from "@lingui/react/macro";
import { confirmClose } from "../lib/unsaved-changes";
import { useNotebookStore } from "../store/notebookStore";
import type { NotebookTab } from "../store/notebookStore";
import { displayTitle } from "../lib/tab-title";
import { nbClose, nbSetActive, createWindow } from "../lib/notebook-commands";

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
  const { t } = useLingui();
  const newTabShortcut = "Cmd+T";
  const notebooks = useNotebookStore((s) => s.notebooks);
  const activeNotebookId = useNotebookStore((s) => s.activeNotebookId);
  const removeTab = useNotebookStore((s) => s.removeTab);
  const setActiveTab = useNotebookStore((s) => s.setActiveTab);
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const contextMenuRef = useRef<HTMLDivElement>(null);

  const tabs = Object.values(notebooks);

  const handleNewTab = useCallback(async () => {
    await openNewNotebook();
  }, []);

  const handleCloseTab = useCallback(async (id: string) => {
    if (tabs.length <= 1) return;
    const tab = notebooks[id];
    // Save, Don't Save or Cancel when it has unsaved changes
    if (!(await confirmClose([id]))) return;
    try {
      // Only close the backend notebook for notebook tabs
      if (!tab || tab.type === "notebook") {
        await nbClose(id);
      }
      removeTab(id);
    } catch (e) {
      console.warn("Failed to close tab:", e);
    }
  }, [tabs.length, notebooks, removeTab]);

  const handleSwitchTab = useCallback(async (id: string) => {
    setActiveTab(id);
    const tab = notebooks[id];
    // Only notify backend for notebook tabs
    if (tab?.type === "notebook") {
      await nbSetActive(id);
    }
  }, [setActiveTab, notebooks]);

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
          {tab.type === "notebook" && (
            <span className={statusDotClass(tab.sessionStatus)} />
          )}
          <span className="tab-title">
            {tab.isDirty ? `${displayTitle(tab.title)} *` : displayTitle(tab.title)}
          </span>
          {tabs.length > 1 && (
            <button
              className="tab-close"
              onClick={(e) => {
                e.stopPropagation();
                handleCloseTab(tab.id);
              }}
              title={t({ message: "Close tab", comment: "Tab bar: tooltip of the x button that closes a notebook tab" })}
            >
              &times;
            </button>
          )}
        </div>
      ))}
      <button
        className="tab-new"
        onClick={handleNewTab}
        title={t({
          message: `New notebook (${newTabShortcut})`,
          comment: "Tab bar: tooltip of the + button that opens a new notebook tab. newTabShortcut is its keyboard shortcut, such as Cmd+T",
        })}
      >
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
            {t({
              message: "Move to New Window",
              comment: "Tab context menu item: moves the notebook tab into a new window",
            })}
          </button>
          <button
            className="tab-context-menu-item"
            disabled={tabs.length <= 1}
            onClick={() => {
              setContextMenu(null);
              handleCloseTab(contextMenu.tabId);
            }}
          >
            {t({ message: "Close Tab", comment: "Tab context menu item: closes the notebook tab" })}
          </button>
        </div>
      )}
    </div>
  );
}
