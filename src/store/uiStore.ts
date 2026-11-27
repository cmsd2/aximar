// The window's panels and dialogs: what is open, so that commands from the
// menu bar, the palette and the toolbar can open and close them
import { create } from "zustand";

interface UiState {
  variablesOpen: boolean;
  docsOpen: boolean;
  /** The function the docs panel was last asked to show */
  docsFunctionName: string | undefined;
  /** Bumped on each request, so asking for the same function twice reloads it */
  docsRequestId: number;
  paletteOpen: boolean;
  paletteQuery: string | undefined;
  templateChooserOpen: boolean;
  exportLatexOpen: boolean;
  shortcutsOpen: boolean;

  setVariablesOpen: (open: boolean) => void;
  setDocsOpen: (open: boolean) => void;
  /** Open the docs panel at a function */
  openDocsFor: (name: string) => void;
  /** Open the palette, with `query` typed in, or close it */
  setPaletteOpen: (open: boolean, query?: string) => void;
  setTemplateChooserOpen: (open: boolean) => void;
  setExportLatexOpen: (open: boolean) => void;
  setShortcutsOpen: (open: boolean) => void;
}

export const useUiStore = create<UiState>((set) => ({
  variablesOpen: false,
  docsOpen: false,
  docsFunctionName: undefined,
  docsRequestId: 0,
  paletteOpen: false,
  paletteQuery: undefined,
  templateChooserOpen: false,
  exportLatexOpen: false,
  shortcutsOpen: false,

  setVariablesOpen: (variablesOpen) => set({ variablesOpen }),
  setDocsOpen: (docsOpen) => set({ docsOpen }),
  openDocsFor: (name) =>
    set((s) => ({ docsOpen: true, docsFunctionName: name, docsRequestId: s.docsRequestId + 1 })),
  setPaletteOpen: (paletteOpen, paletteQuery) => set({ paletteOpen, paletteQuery: paletteOpen ? paletteQuery : undefined }),
  setTemplateChooserOpen: (templateChooserOpen) => set({ templateChooserOpen }),
  setExportLatexOpen: (exportLatexOpen) => set({ exportLatexOpen }),
  setShortcutsOpen: (shortcutsOpen) => set({ shortcutsOpen }),
}));
