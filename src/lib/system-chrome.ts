// The desktop's chrome, for the tab bar and toolbar under the title bar.
//
// On Linux the system draws the title bar and the GTK menu bar in the
// desktop theme's colours, and the page's tabs sit right below them.  The
// backend reads the menu bar's colours (src-tauri/src/commands/
// system_chrome.rs) and the page paints its chrome with them, so the frame,
// menus, tabs and toolbar read as one band.  macOS and Windows send none.
// See docs/design-library.md.

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
}

/** The variables the page sets on the root element, so it can clear them all */
export const SYSTEM_CHROME_VARIABLES = ["--chrome-bg", "--chrome-border", "--font-ui"] as const;

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
    if (chrome.border) out["--chrome-border"] = chrome.border;
  }
  return out;
}
