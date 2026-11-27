import { useCallback, useEffect, useState, useRef } from "react";
import { openNewNotebook } from "../lib/new-notebook";
import { command } from "../commands/registry";
import { formatShortcut, matches } from "../commands/shortcut";
import { closeTab, moveTabToNewWindow, stepTab, switchTab } from "../lib/tab-commands";
import { useLingui } from "@lingui/react/macro";
import { IconPlus, IconX } from "./ui/icons";
import { useNotebookStore } from "../store/notebookStore";
import type { NotebookTab } from "../store/notebookStore";
import { displayTitle } from "../lib/tab-title";

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
  const newTabShortcut = formatShortcut(command("file.new")!.shortcut!);
  const notebooks = useNotebookStore((s) => s.notebooks);
  const activeNotebookId = useNotebookStore((s) => s.activeNotebookId);
  const [contextMenu, setContextMenu] = useState<ContextMenuState | null>(null);
  const contextMenuRef = useRef<HTMLDivElement>(null);

  const tabs = Object.values(notebooks);

  const handleNewTab = useCallback(async () => {
    await openNewNotebook();
  }, []);

  const handleCloseTab = useCallback((id: string) => closeTab(id), []);

  const handleSwitchTab = useCallback((id: string) => switchTab(id), []);

  const handleMoveToNewWindow = useCallback(async (tabId: string) => {
    setContextMenu(null);
    await moveTabToNewWindow(tabId);
  }, []);

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

  // Ctrl+Tab and Ctrl+Shift+Tab switch tabs too; a menu item has one
  // shortcut, so these two are caught here.  The rest are the menu's.
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      const dir = matches("Ctrl+Tab", e) ? 1 : matches("Ctrl+Shift+Tab", e) ? -1 : 0;
      if (!dir) return;
      e.preventDefault();
      void stepTab(dir);
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

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
              <IconX size={12} stroke={1.75} aria-hidden="true" />
            </button>
          )}
        </div>
      ))}
      <button
        className="tab-new"
        onClick={handleNewTab}
        title={t({
          message: `New notebook (${newTabShortcut})`,
          comment: "Tab bar: tooltip of the + button that opens a new notebook tab. newTabShortcut is its keyboard shortcut in the platform's names, such as ⌘N or Ctrl+N",
        })}
      >
        <IconPlus size={14} stroke={1.75} aria-hidden="true" />
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
