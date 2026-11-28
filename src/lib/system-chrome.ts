// The desktop's chrome, for the page's title bar, tab bar and toolbar.
//
// On Linux the backend reads how the desktop draws a window's frame
// (src-tauri/src/commands/system_chrome.rs): the GTK menu bar's colours,
// the title bar's colours, the title bar's button layout and which desktop
// it is.  With the system's title bar, the tab bar and toolbar take the
// menu bar's colours, so the frame, menus, tabs and toolbar read as one
// band.  Where the page draws the title bar, it takes the title bar's
// colours and lays out its buttons as the desktop does (TitleBar.tsx).
// macOS and Windows send none.  See docs/design-library.md.

import { create } from "zustand";
import type { Theme } from "../store/notebookStore";

/** How the desktop draws a window's menu bar, as CSS colours */
export interface SystemChrome {
  /** The menu bar's background while the window is active */
  background: string;
  /** Its background while another window is active */
  backdropBackground: string;
  /** The theme's border colour, if it names one */
  border: string | null;
  /** Whether the background is dark */
  dark: boolean;
  /** The desktop's interface font, such as "Noto Sans" */
  fontFamily: string | null;
  /** The title bar's background while the window is active */
  titleBackground: string;
  /** The title bar's background while another window is active */
  titleBackdropBackground: string;
  /** GTK's `gtk-decoration-layout`, such as "icon:minimize,maximize,close" */
  decorationLayout: string | null;
  /** The desktop, for the window buttons' look */
  desktop: "kde" | "gnome" | "other";
}

/** The desktop's chrome, once the backend has sent it */
export const useSystemChromeStore = create<{ chrome: SystemChrome | null }>(() => ({ chrome: null }));

/** The variables the page sets on the root element, so it can clear them all */
export const SYSTEM_CHROME_VARIABLES = ["--chrome-bg", "--chrome-border", "--chrome-title-bg", "--font-ui"] as const;

export type SystemChromeVariable = (typeof SYSTEM_CHROME_VARIABLES)[number];

/** Themes with tinted greys of their own, which the desktop's would clash with */
const OWN_CHROME: ReadonlySet<Theme> = new Set(["paper", "night"]);

/** A font family as a CSS string, ahead of the generic family */
function fontStack(family: string): string {
  return `"${family.replace(/["\\]/g, "\\$&")}", sans-serif`;
}

/**
 * The variables to set for the desktop's chrome.  The colours apply only
 * when they are as light or dark as the page: a light theme's menu bar
 * above a dark notebook would leave the tabs light and their text unreadable,
 * so then the page keeps its own.  Paper and Night keep their own tinted
 * greys.  The font applies in every theme.
 */
export function systemChromeVariables(
  chrome: SystemChrome | null,
  { theme, systemDark, focused }: { theme: Theme; systemDark: boolean; focused: boolean },
): Partial<Record<SystemChromeVariable, string>> {
  if (!chrome) return {};
  const out: Partial<Record<SystemChromeVariable, string>> = {};
  if (chrome.fontFamily) out["--font-ui"] = fontStack(chrome.fontFamily);
  const pageDark = theme === "dark" || (theme === "auto" && systemDark);
  if (!OWN_CHROME.has(theme) && chrome.dark === pageDark) {
    out["--chrome-bg"] = focused ? chrome.background : chrome.backdropBackground;
    out["--chrome-title-bg"] = focused ? chrome.titleBackground : chrome.titleBackdropBackground;
    if (chrome.border) out["--chrome-border"] = chrome.border;
  }
  return out;
}
