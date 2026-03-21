import { create } from "zustand";
import { nanoid } from "nanoid";
import type { LogEntry, LogLevel } from "../types/log";

interface LogState {
  entries: LogEntry[];
  unreadCount: number;
  logOpen: boolean;

  addEntry: (level: LogLevel, message: string, source: string) => void;
  toggleLog: () => void;
  clearLog: () => void;
}

export const useLogStore = create<LogState>((set, get) => ({
  entries: [],
  unreadCount: 0,
  logOpen: false,

  addEntry: (level, message, source) => {
    const entry: LogEntry = {
      id: nanoid(),
      timestamp: Date.now(),
      level,
      message,
      source,
    };
    set((state) => ({
      entries: [...state.entries, entry],
      unreadCount: state.logOpen ? state.unreadCount : state.unreadCount + 1,
    }));
  },

  toggleLog: () => {
    const opening = !get().logOpen;
    set({
      logOpen: opening,
      unreadCount: opening ? 0 : get().unreadCount,
    });
  },

  clearLog: () => set({ entries: [], unreadCount: 0 }),
}));
