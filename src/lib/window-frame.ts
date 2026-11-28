// Who draws the window's title bar and menu bar.  On Windows the page does,
// with the tabs in the title bar, as Windows 11's own apps do: the system's
// menu bar can't take the window's fonts or colours, and its title bar sits
// above the tabs (src-tauri/src/commands/window.rs).  Elsewhere the system
// draws both, from the menu the window sends (useMenuBar.ts).
import { currentPlatform, type Platform } from "../commands/shortcut";

/** Whether the page draws the title bar and the menu bar */
export function drawsWindowFrame(platform: Platform = currentPlatform()): boolean {
  return platform === "windows";
}
