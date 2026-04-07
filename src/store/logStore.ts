import { create } from "zustand";
import { nanoid } from "nanoid";
import type { LogEntry, LogLevel, RawOutputEntry, LogTab } from "../types/log";

const MAX_ENTRIES = 5000;
const MAX_RAW_ENTRIES = 5000;

interface LogState {
  entries: LogEntry[];
  rawOutput: RawOutputEntry[];
  unreadCount: number;
  windowOpen: boolean;
  activeTab: LogTab;

  addEntry: (level: LogLevel, message: string, source: string) => void;
  addRawOutput: (line: string, stream: "stdin" | "stdout" | "stderr", timestamp: number) => void;
  toggleWindow: () => void;
  openWindow: () => void;
  closeWindow: () => void;
  setActiveTab: (tab: LogTab) => void;
  clearLog: () => void;
  clearRawOutput: () => void;
}

export const useLogStore = create<LogState>((set, get) => ({
  entries: [],
  rawOutput: [],
  unreadCount: 0,
  windowOpen: false,
  activeTab: "app",

  addEntry: (level, message, source) => {
    const entry: LogEntry = {
      id: nanoid(),
      timestamp: Date.now(),
      level,
      message,
      source,
    };
    set((state) => {
      const next = [...state.entries, entry];
      if (next.length > MAX_ENTRIES) {
        next.splice(0, next.length - MAX_ENTRIES);
      }
      return {
        entries: next,
        unreadCount:
          level === "error" && !state.windowOpen
            ? state.unreadCount + 1
            : state.unreadCount,
      };
    });
  },

  addRawOutput: (line, stream, timestamp) => {
    const entry: RawOutputEntry = {
      id: nanoid(),
      line,
      stream,
      timestamp,
    };
    set((state) => {
      const next = [...state.rawOutput, entry];
      // Cap buffer size
      if (next.length > MAX_RAW_ENTRIES) {
        next.splice(0, next.length - MAX_RAW_ENTRIES);
      }
      return { rawOutput: next };
    });
  },

  toggleWindow: () => {
    const opening = !get().windowOpen;
    set({
      windowOpen: opening,
      unreadCount: opening ? 0 : get().unreadCount,
    });
  },

  openWindow: () => {
    set({ windowOpen: true, unreadCount: 0 });
  },

  closeWindow: () => {
    set({ windowOpen: false });
  },

  setActiveTab: (tab) => set({ activeTab: tab }),

  clearLog: () => set({ entries: [], unreadCount: 0 }),

  clearRawOutput: () => set({ rawOutput: [] }),
}));
