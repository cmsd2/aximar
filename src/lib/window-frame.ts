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
// On Linux the system draws both, from the menu the window sends
// (useMenuBar.ts).  See src-tauri/src/commands/window.rs.
import { currentPlatform, type Platform } from "../commands/shortcut";

/** Whether the page draws the title bar, with the tabs in it */
export function drawsTitleBar(platform: Platform = currentPlatform()): boolean {
  return platform === "windows" || platform === "macos";
}

/** Whether the page draws the menu bar and the window's buttons */
export function drawsMenuBar(platform: Platform = currentPlatform()): boolean {
  return platform === "windows";
}
