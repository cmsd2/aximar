// Who draws the window's title bar and menu bar.
//
// On Windows the page draws both, with the tabs in the title bar, as
// Windows 11's own apps do: the system's menu bar can't take the window's
// fonts or colours, and its title bar sits above the tabs.
//
// On macOS the system draws the menu bar, at the top of the screen, and
// the window's traffic lights; the page draws the rest of the title bar
// around them, with the tabs in it, as Safari and Xcode do.
//
// On Linux the page draws both too, as Firefox does, unless the user chose
// the system's title bar in Settings: then the system draws both, from the
// menu the window sends (useMenuBar.ts).  The backend tells the page
// before its scripts run.  See src-tauri/src/commands/window.rs.
import { currentPlatform, type Platform } from "../commands/shortcut";

declare global {
  interface Window {
    /** Set by the backend on Linux when the page draws the frame */
    __AXIMAR_PAGE_FRAME__?: boolean;
  }
}

/** Whether the page draws Linux's frame: the backend says so */
function linuxPageFrame(): boolean {
  return typeof window !== "undefined" && window.__AXIMAR_PAGE_FRAME__ === true;
}

/** Whether the page draws the title bar, with the tabs in it */
export function drawsTitleBar(platform: Platform = currentPlatform()): boolean {
  if (platform === "linux") return linuxPageFrame();
  return platform === "windows" || platform === "macos";
}

/** Whether the page draws the menu bar and the window's buttons */
export function drawsMenuBar(platform: Platform = currentPlatform()): boolean {
  if (platform === "linux") return linuxPageFrame();
  return platform === "windows";
}

/** A title bar control, as GTK names it in `gtk-decoration-layout` */
export type TitleBarItem = "icon" | "minimize" | "maximize" | "close";

/** What sits at each end of the title bar, outside the tabs */
export interface TitleBarLayout {
  start: TitleBarItem[];
  end: TitleBarItem[];
}

/** Windows' layout, and Linux's until the desktop says otherwise */
export const DEFAULT_LAYOUT: TitleBarLayout = { start: ["icon"], end: ["minimize", "maximize", "close"] };

const ITEMS: readonly string[] = ["icon", "minimize", "maximize", "close"];

/**
 * The layout from GTK's `gtk-decoration-layout`, such as
 * "icon:minimize,maximize,close" (KDE) or "appmenu:close" (GNOME): the
 * items before the colon go at the start, the rest at the end.  Items the
 * page doesn't draw, such as GNOME's app menu, are left out.
 */
export function parseDecorationLayout(layout: string | null | undefined): TitleBarLayout {
  if (layout == null || !layout.includes(":")) return DEFAULT_LAYOUT;
  const side = (s: string) =>
    s
      .split(",")
      .map((i) => i.trim())
      .filter((i): i is TitleBarItem => ITEMS.includes(i));
  const [start, end] = layout.split(":", 2);
  return { start: side(start), end: side(end) };
}
