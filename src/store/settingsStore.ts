// Whether the Settings window is open, and at which section.  Anything in
// the window can open it at a section, such as the Maxima Not Found dialog
// opening Maxima.
import { create } from "zustand";
import type { SectionId } from "../components/settings/sections";

interface SettingsState {
  open: boolean;
  /** The section asked for; null shows the one shown last */
  section: SectionId | null;
  /** Counts requests, so asking again for the same section still moves there */
  request: number;
  /** Open Settings, at `section` or else at the section shown last */
  openSettings: (section?: SectionId) => void;
  closeSettings: () => void;
}

export const useSettingsStore = create<SettingsState>((set) => ({
  open: false,
  section: null,
  request: 0,
  openSettings: (section) => set((s) => ({ open: true, section: section ?? null, request: s.request + 1 })),
  closeSettings: () => set({ open: false, section: null }),
}));
