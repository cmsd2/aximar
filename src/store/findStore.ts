import { create } from "zustand";

export interface FindMatch {
  cellId: string;
  start: number;
  end: number;
}

interface FindState {
  isOpen: boolean;
  replaceVisible: boolean;
  query: string;
  replacement: string;
  caseSensitive: boolean;
  matches: FindMatch[];
  currentMatchIndex: number;
  navigateTo: { cellId: string; start: number; end: number } | null;

  open: (withReplace?: boolean) => void;
  close: () => void;
  setQuery: (query: string) => void;
  setReplacement: (replacement: string) => void;
  toggleCaseSensitive: () => void;
  toggleReplaceVisible: () => void;
  setMatches: (matches: FindMatch[]) => void;
  goToNextMatch: () => void;
  goToPrevMatch: () => void;
  setNavigateTo: (nav: { cellId: string; start: number; end: number }) => void;
  clearNavigateTo: () => void;
}

export const useFindStore = create<FindState>((set) => ({
  isOpen: false,
  replaceVisible: false,
  query: "",
  replacement: "",
  caseSensitive: false,
  matches: [],
  currentMatchIndex: 0,
  navigateTo: null,

  open: (withReplace?: boolean) =>
    set((state) => ({
      isOpen: true,
      replaceVisible: withReplace ?? state.replaceVisible,
    })),

  close: () =>
    set({
      isOpen: false,
      matches: [],
      currentMatchIndex: 0,
    }),

  setQuery: (query: string) => set({ query }),

  setReplacement: (replacement: string) => set({ replacement }),

  toggleCaseSensitive: () =>
    set((state) => ({ caseSensitive: !state.caseSensitive })),

  toggleReplaceVisible: () =>
    set((state) => ({ replaceVisible: !state.replaceVisible })),

  setMatches: (matches: FindMatch[]) =>
    set((state) => ({
      matches,
      currentMatchIndex: matches.length > 0
        ? Math.min(state.currentMatchIndex, matches.length - 1)
        : 0,
    })),

  goToNextMatch: () =>
    set((state) => {
      if (state.matches.length === 0) return state;
      return { currentMatchIndex: (state.currentMatchIndex + 1) % state.matches.length };
    }),

  goToPrevMatch: () =>
    set((state) => {
      if (state.matches.length === 0) return state;
      return {
        currentMatchIndex:
          (state.currentMatchIndex - 1 + state.matches.length) % state.matches.length,
      };
    }),

  setNavigateTo: (nav: { cellId: string; start: number; end: number }) =>
    set({ navigateTo: nav }),

  clearNavigateTo: () => set({ navigateTo: null }),
}));
