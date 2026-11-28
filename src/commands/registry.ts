// The window's commands: one list, from which the menu bar, the palette's
// commands, the cell context menu, the toolbar's tooltips, the shortcut
// hints and the Keyboard Shortcuts list are all built.  A command that
// changes the notebook still goes through the backend's notebook commands
// (ADR 007); this list only says where it is reached from.
import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { invoke } from "@tauri-apps/api/core";
import { openUrl } from "@tauri-apps/plugin-opener";
import { getActiveTabState, useNotebookStore } from "../store/notebookStore";
import { useUiStore } from "../store/uiStore";
import { useLogStore } from "../store/logStore";
import { useSettingsStore } from "../store/settingsStore";
import { useFindStore } from "../store/findStore";
import { nbDeleteCell, nbMoveCell, nbRedo, nbUndo, createWindow } from "../lib/notebook-commands";
import { cancelEvaluation } from "../lib/maxima-client";
import { flushNotebook } from "../lib/dirty-inputs";
import { afterRun, goTo, insertAt } from "../lib/cell-caret";
import { openNewNotebook } from "../lib/new-notebook";
import { openFromDisk, saveActive, saveActiveAs } from "../lib/file-commands";
import { closeActiveTab, closeWindow, moveTabToNewWindow, stepTab } from "../lib/tab-commands";
import { currentPlatform, matches, type Platform, type Shortcut } from "./shortcut";

export type MenuName = "app" | "file" | "edit" | "cell" | "session" | "view" | "window" | "help";

/** What the commands need from React: the session functions useMaxima gives */
export interface CommandContext {
  /** The cell a context-menu command acts on; otherwise the active cell */
  cellId?: string;
  executeCell: (cellId: string, input: string) => Promise<boolean>;
  runAll: (notebookId: string) => Promise<unknown>;
  restartSession: () => Promise<unknown>;
}

/** The window's state that decides which commands are enabled or checked */
export interface CommandState {
  isNotebook: boolean;
  activeCellId: string | null;
  activeCellType: string | null;
  activeIsFirst: boolean;
  activeIsLast: boolean;
  cellCount: number;
  canUndo: boolean;
  canRedo: boolean;
  /** Focus is in a text field outside the cells, which undoes its own typing */
  inTextField: boolean;
  running: boolean;
  tabCount: number;
  variablesOpen: boolean;
  docsOpen: boolean;
  logOpen: boolean;
}

export interface Command {
  id: string;
  label: MessageDescriptor;
  /** The label in the cell context menu, where it differs */
  contextLabel?: MessageDescriptor;
  /** The menu the command is in, and its group there (groups are separated) */
  menu?: { name: MenuName; group: number };
  /** In the cell context menu, in this order */
  context?: number;
  /** Bound as the menu item's accelerator */
  shortcut?: Shortcut;
  /** Handled by the cell editor: shown, never bound */
  editorShortcut?: Shortcut;
  /** Listed in the hints shown while Cmd or Ctrl is held */
  hint?: boolean;
  enabled?: (s: CommandState) => boolean;
  checked?: (s: CommandState) => boolean;
  run: (ctx: CommandContext) => void | Promise<void>;
}

/** The cell a command acts on: the context menu's, or the active one */
function targetCell(ctx: CommandContext): string | null {
  return ctx.cellId ?? getActiveTabState().activeCellId;
}

/** Run a code cell; then, like Shift+Enter, go on to the next cell or the caret below */
async function runCell(ctx: CommandContext, advance: boolean): Promise<void> {
  const id = targetCell(ctx);
  const cell = getActiveTabState().cells.find((c) => c.id === id);
  if (!cell || cell.cellType !== "code") return;
  const ok = await ctx.executeCell(cell.id, cell.input);
  if (ok && advance) goTo(afterRun(getActiveTabState().cells, cell.id));
}

/** Insert a cell beside the target cell, or at the end */
function insertNear(ctx: CommandContext, type: "code" | "markdown", where: "above" | "below"): Promise<string> {
  const cells = getActiveTabState().cells;
  const id = targetCell(ctx) ?? cells[cells.length - 1]?.id ?? null;
  const i = cells.findIndex((c) => c.id === id);
  const after = where === "below" ? id : i > 0 ? cells[i - 1].id : null;
  return insertAt({ after }, type);
}

const RELEASES = "https://github.com/cmsd2/aximar/releases";
const NEW_ISSUE = "https://github.com/cmsd2/aximar/issues/new";

const hasCell = (s: CommandState) => s.isNotebook && s.activeCellId !== null;
const codeCell = (s: CommandState) => hasCell(s) && s.activeCellType === "code";

export const COMMANDS: Command[] = [
  // Aximar (macOS); Settings is in Edit elsewhere
  {
    id: "app.settings",
    label: msg({ message: "Settings...", comment: "Menu item: opens the Settings window. In the Aximar menu on macOS, the Edit menu elsewhere. max-length:24" }),
    menu: { name: "app", group: 1 },
    shortcut: "Mod+,",
    run: () => useSettingsStore.getState().openSettings(),
  },

  // File
  {
    id: "file.new",
    label: msg({ message: "New Notebook", comment: "File menu item: opens an empty notebook in a new tab. max-length:30" }),
    menu: { name: "file", group: 1 },
    shortcut: "Mod+N",
    run: () => void openNewNotebook(),
  },
  {
    id: "file.new_window",
    label: msg({ message: "New Window", comment: "File menu item: opens a new window. max-length:30" }),
    menu: { name: "file", group: 1 },
    shortcut: "Mod+Shift+N",
    run: () => void createWindow(),
  },
  {
    id: "file.new_from_template",
    label: msg({ message: "New from Template...", comment: "File menu item: opens the starter notebook chooser. max-length:30" }),
    menu: { name: "file", group: 1 },
    run: () => useUiStore.getState().setTemplateChooserOpen(true),
  },
  {
    id: "file.open",
    label: msg({ message: "Open...", comment: "File menu item: opens a notebook or plot file. max-length:30" }),
    menu: { name: "file", group: 1 },
    shortcut: "Mod+O",
    run: () => openFromDisk(),
  },
  {
    id: "file.save",
    label: msg({ message: "Save", comment: "File menu item: saves the notebook. max-length:30" }),
    menu: { name: "file", group: 2 },
    shortcut: "Mod+S",
    run: () => saveActive(),
  },
  {
    id: "file.save_as",
    label: msg({ message: "Save As...", comment: "File menu item: saves the notebook under a new name. max-length:30" }),
    menu: { name: "file", group: 2 },
    shortcut: "Mod+Shift+S",
    run: () => saveActiveAs(),
  },
  {
    id: "file.export_latex",
    label: msg({ message: "Export as LaTeX...", comment: "File menu item: exports the notebook as a LaTeX document. max-length:30" }),
    menu: { name: "file", group: 3 },
    shortcut: "Mod+Shift+E",
    enabled: (s) => s.isNotebook,
    run: () => useUiStore.getState().setExportLatexOpen(true),
  },
  {
    id: "file.print",
    label: msg({ message: "Print...", comment: "File menu item: prints the notebook. max-length:30" }),
    menu: { name: "file", group: 3 },
    shortcut: "Mod+P",
    run: () => void invoke("print_window"),
  },
  {
    id: "file.close_tab",
    label: msg({ message: "Close Tab", comment: "File menu item: closes the notebook tab, or the window when it is the last tab. max-length:30" }),
    menu: { name: "file", group: 4 },
    shortcut: "Mod+W",
    run: () => closeActiveTab(),
  },
  {
    id: "file.close_window",
    label: msg({ message: "Close Window", comment: "File menu item: closes the window and its tabs. max-length:30" }),
    menu: { name: "file", group: 4 },
    shortcut: "Mod+Shift+W",
    run: () => closeWindow(),
  },

  // Edit; Cut, Copy, Paste and Select All are the system's
  {
    id: "edit.undo",
    label: msg({ message: "Undo", comment: "Edit menu item: undoes the last change to the notebook, or to the text field being typed in. max-length:24" }),
    menu: { name: "edit", group: 1 },
    shortcut: "Mod+Z",
    hint: true,
    enabled: (s) => s.canUndo || s.inTextField,
    run: () => undoOrRedo("undo"),
  },
  {
    id: "edit.redo",
    label: msg({ message: "Redo", comment: "Edit menu item: redoes the last undone change. max-length:24" }),
    menu: { name: "edit", group: 1 },
    shortcut: "Mod+Shift+Z",
    hint: true,
    enabled: (s) => s.canRedo || s.inTextField,
    run: () => undoOrRedo("redo"),
  },
  {
    id: "edit.find",
    label: msg({ message: "Find", comment: "Edit menu item: opens the find bar. max-length:24" }),
    menu: { name: "edit", group: 3 },
    shortcut: "Mod+F",
    hint: true,
    enabled: (s) => s.isNotebook,
    run: () => useFindStore.getState().open(false),
  },
  {
    id: "edit.find_replace",
    label: msg({ message: "Find and Replace", comment: "Edit menu item: opens the find bar with its replace field. max-length:24" }),
    menu: { name: "edit", group: 3 },
    shortcut: "Mod+Shift+F",
    hint: true,
    enabled: (s) => s.isNotebook,
    run: () => useFindStore.getState().open(true),
  },

  // Cell
  {
    id: "cell.new_code",
    label: msg({ message: "New Code Cell", comment: "Cell menu item: inserts a Maxima code cell below the active cell. max-length:30" }),
    contextLabel: msg({ message: "New Code Cell Below", comment: "Cell context menu item: inserts a Maxima code cell below this cell. max-length:30" }),
    menu: { name: "cell", group: 1 },
    context: 4,
    enabled: (s) => s.isNotebook,
    run: (ctx) => void insertNear(ctx, "code", "below"),
  },
  {
    id: "cell.new_code_above",
    label: msg({ message: "New Code Cell Above", comment: "Cell context menu item: inserts a Maxima code cell above this cell. max-length:30" }),
    context: 3,
    enabled: (s) => s.isNotebook,
    run: (ctx) => void insertNear(ctx, "code", "above"),
  },
  {
    id: "cell.new_markdown",
    label: msg({ message: "New Markdown Cell", comment: "Cell menu item: inserts a Markdown text cell below the active cell. max-length:30" }),
    contextLabel: msg({ message: "New Markdown Cell Below", comment: "Cell context menu item: inserts a Markdown text cell below this cell. max-length:30" }),
    menu: { name: "cell", group: 1 },
    context: 5,
    enabled: (s) => s.isNotebook,
    run: (ctx) => void insertNear(ctx, "markdown", "below"),
  },
  {
    id: "cell.run",
    label: msg({ message: "Run", comment: "Cell menu item: runs the cell and moves on to the next one. max-length:30" }),
    menu: { name: "cell", group: 2 },
    context: 1,
    editorShortcut: "Shift+Enter",
    enabled: codeCell,
    run: (ctx) => runCell(ctx, true),
  },
  {
    id: "cell.run_stay",
    label: msg({ message: "Run and Stay", comment: "Cell menu item: runs the cell and keeps the cursor in it. max-length:30" }),
    menu: { name: "cell", group: 2 },
    context: 2,
    editorShortcut: "Mod+Enter",
    enabled: codeCell,
    run: (ctx) => runCell(ctx, false),
  },
  {
    id: "cell.run_all",
    label: msg({ message: "Run All", comment: "Cell menu item and toolbar button: runs every cell in order. max-length:30" }),
    menu: { name: "cell", group: 2 },
    enabled: (s) => s.isNotebook && s.cellCount > 0,
    run: async (ctx) => {
      const id = useNotebookStore.getState().activeNotebookId;
      if (id) await ctx.runAll(id);
    },
  },
  {
    id: "cell.move_up",
    label: msg({ message: "Move Up", comment: "Cell menu item: moves the cell up one place. max-length:30" }),
    menu: { name: "cell", group: 3 },
    context: 6,
    // Not Mod+Shift+Up: in a cell the editor takes that, to select to the
    // start.  The cell editor gives Mod+Alt+Up up (useCodeMirrorEditor.ts)
    shortcut: "Mod+Alt+ArrowUp",
    hint: true,
    enabled: (s) => hasCell(s) && !s.activeIsFirst,
    run: (ctx) => {
      const id = targetCell(ctx);
      if (id) void nbMoveCell(id, "up");
    },
  },
  {
    id: "cell.move_down",
    label: msg({ message: "Move Down", comment: "Cell menu item: moves the cell down one place. max-length:30" }),
    menu: { name: "cell", group: 3 },
    context: 7,
    shortcut: "Mod+Alt+ArrowDown",
    hint: true,
    enabled: (s) => hasCell(s) && !s.activeIsLast,
    run: (ctx) => {
      const id = targetCell(ctx);
      if (id) void nbMoveCell(id, "down");
    },
  },
  {
    id: "cell.delete",
    label: msg({ message: "Delete Cell", comment: "Cell menu item: deletes the cell; Undo brings it back. max-length:30" }),
    menu: { name: "cell", group: 4 },
    context: 8,
    shortcut: "Mod+Shift+Backspace",
    hint: true,
    enabled: (s) => hasCell(s) && s.cellCount > 1,
    run: (ctx) => {
      const id = targetCell(ctx);
      if (id && getActiveTabState().cells.length > 1) void nbDeleteCell(id);
    },
  },

  // Session
  {
    id: "session.interrupt",
    label: msg({
      message: "Interrupt",
      comment: "Session menu item and toolbar button: asks the running cell to stop. It works only when the computation checks for it; Restart always stops Maxima. max-length:24",
    }),
    menu: { name: "session", group: 1 },
    shortcut: "Mod+.",
    enabled: (s) => s.running,
    run: () => void cancelEvaluation(),
  },
  {
    id: "session.restart",
    label: msg({ message: "Restart", comment: "Session menu item and toolbar button: restarts Maxima, clearing its variables. max-length:24" }),
    menu: { name: "session", group: 1 },
    run: async (ctx) => {
      await ctx.restartSession();
    },
  },

  // View
  {
    id: "view.variables",
    label: msg({ message: "Variables", comment: "View menu item and toolbar toggle: shows or hides the variables panel. max-length:24" }),
    menu: { name: "view", group: 1 },
    enabled: (s) => s.isNotebook,
    checked: (s) => s.variablesOpen,
    run: () => {
      const ui = useUiStore.getState();
      ui.setVariablesOpen(!ui.variablesOpen);
    },
  },
  {
    id: "view.docs",
    label: msg({ message: "Docs", comment: "View menu item and toolbar toggle: shows or hides the documentation panel. max-length:24" }),
    menu: { name: "view", group: 1 },
    enabled: (s) => s.isNotebook,
    checked: (s) => s.docsOpen,
    run: () => {
      const ui = useUiStore.getState();
      ui.setDocsOpen(!ui.docsOpen);
    },
  },
  {
    id: "view.log",
    label: msg({ message: "Log", comment: "View menu item and toolbar toggle: shows or hides the log panel. max-length:24" }),
    menu: { name: "view", group: 1 },
    checked: (s) => s.logOpen,
    run: () => useLogStore.getState().toggleWindow(),
  },
  {
    id: "view.palette",
    label: msg({ message: "Command Palette", comment: "View menu item: opens the palette, which finds commands, functions and symbols. max-length:24" }),
    menu: { name: "view", group: 2 },
    shortcut: "Mod+K",
    hint: true,
    run: () => {
      const ui = useUiStore.getState();
      ui.setPaletteOpen(!ui.paletteOpen);
    },
  },

  // Window; Minimize and Zoom are the system's
  {
    id: "window.next_tab",
    label: msg({ message: "Next Tab", comment: "Window menu item: switches to the next notebook tab. max-length:30" }),
    menu: { name: "window", group: 2 },
    shortcut: "Mod+Shift+]",
    enabled: (s) => s.tabCount > 1,
    run: () => stepTab(1),
  },
  {
    id: "window.prev_tab",
    label: msg({ message: "Previous Tab", comment: "Window menu item: switches to the previous notebook tab. max-length:30" }),
    menu: { name: "window", group: 2 },
    shortcut: "Mod+Shift+[",
    enabled: (s) => s.tabCount > 1,
    run: () => stepTab(-1),
  },
  {
    id: "window.move_tab",
    label: msg({ message: "Move Tab to New Window", comment: "Window menu item: moves the active tab to a window of its own. max-length:30" }),
    menu: { name: "window", group: 2 },
    enabled: (s) => s.tabCount > 1,
    run: () => {
      const id = useNotebookStore.getState().activeNotebookId;
      if (id) void moveTabToNewWindow(id);
    },
  },

  // Help
  {
    id: "help.docs",
    label: msg({ message: "Aximar Help", comment: "Help menu item: opens the documentation panel. max-length:30" }),
    menu: { name: "help", group: 1 },
    enabled: (s) => s.isNotebook,
    run: () => useUiStore.getState().setDocsOpen(true),
  },
  {
    id: "help.shortcuts",
    label: msg({ message: "Keyboard Shortcuts", comment: "Help menu item: lists every keyboard shortcut. max-length:30" }),
    menu: { name: "help", group: 1 },
    shortcut: "Mod+/",
    run: () => useUiStore.getState().setShortcutsOpen(true),
  },
  {
    id: "help.whats_new",
    label: msg({ message: "What's New", comment: "Help menu item: opens the release notes on GitHub. max-length:30" }),
    menu: { name: "help", group: 2 },
    run: () => void openUrl(RELEASES),
  },
  {
    id: "help.report_issue",
    label: msg({ message: "Report an Issue", comment: "Help menu item: opens a new issue on Aximar's GitHub page. max-length:30" }),
    menu: { name: "help", group: 2 },
    run: () => void openUrl(NEW_ISSUE),
  },
];

const BY_ID = new Map(COMMANDS.map((c) => [c.id, c]));

/** The command with this id */
export function command(id: string): Command | undefined {
  return BY_ID.get(id);
}

/** The command whose menu shortcut a key press is, on this platform */
export function shortcutCommand(
  e: Parameters<typeof matches>[1],
  platform: Platform = currentPlatform(),
): Command | undefined {
  return COMMANDS.find((c) => c.shortcut && matches(c.shortcut, e, platform));
}

/** Whether focus is in a text field outside the cells, which has its own undo */
function inOwnTextField(): boolean {
  if (typeof document === "undefined") return false;
  const el = document.activeElement;
  if (!el || !(el instanceof HTMLInputElement || el instanceof HTMLTextAreaElement)) return false;
  // A Markdown cell's textarea and the caret's input belong to the notebook
  return !el.closest(".cell, .cell-gap");
}

/** How soon after one asker the other counts as the same key press */
const ECHO_MS = 250;
let lastUndo: { which: "undo" | "redo"; from: "menu" | "editor"; at: number } | null = null;

/**
 * Undo or redo the notebook's last change, as the backend records it, or,
 * in a text field outside the cells, that field's typing.  Typing reaches
 * the backend after a pause, so it is sent first: otherwise Undo would
 * skip it, and sending it later would put it back.
 *
 * `from` says what asked.  A cell's editor runs undo itself, because on
 * Windows Ctrl+Z goes to the page, not the menu, and the browser's own
 * undo garbles the editor (useCodeMirrorEditor.ts).  Should one key press
 * reach both, the second asker is ignored; repeats from one asker aren't.
 */
export function undoOrRedo(which: "undo" | "redo", from: "menu" | "editor" = "menu"): void {
  const now = Date.now();
  const echo = lastUndo && lastUndo.which === which && lastUndo.from !== from && now - lastUndo.at < ECHO_MS;
  lastUndo = { which, from, at: now };
  if (echo) return;
  if (inOwnTextField()) {
    document.execCommand(which);
    return;
  }
  const notebookId = useNotebookStore.getState().activeNotebookId;
  void (async () => {
    if (notebookId) await flushNotebook(notebookId);
    await (which === "undo" ? nbUndo() : nbRedo());
  })();
}

/** The state commands are enabled and checked by, read from the stores now */
export function readCommandState(): CommandState {
  const nb = useNotebookStore.getState();
  const id = nb.activeNotebookId;
  const tab = id ? nb.notebooks[id] : undefined;
  const isNotebook = tab?.type === "notebook";
  const cells = isNotebook ? tab.cells : [];
  const activeCellId = isNotebook ? tab.activeCellId : null;
  const index = cells.findIndex((c) => c.id === activeCellId);
  const ui = useUiStore.getState();
  return {
    isNotebook,
    activeCellId: index === -1 ? null : activeCellId,
    activeCellType: index === -1 ? null : cells[index].cellType,
    activeIsFirst: index === 0,
    activeIsLast: index === cells.length - 1,
    cellCount: cells.length,
    canUndo: isNotebook && tab.canUndo,
    canRedo: isNotebook && tab.canRedo,
    inTextField: inOwnTextField(),
    running: isNotebook && (tab.sessionStatus === "Busy" || cells.some((c) => c.status === "running")),
    tabCount: Object.keys(nb.notebooks).length,
    variablesOpen: ui.variablesOpen,
    docsOpen: ui.docsOpen,
    logOpen: useLogStore.getState().windowOpen,
  };
}

/** Whether a command can run in this state */
export function isEnabled(c: Command, s: CommandState): boolean {
  return c.enabled ? c.enabled(s) : true;
}
