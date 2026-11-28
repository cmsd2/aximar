import { useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import appIcon from "../../src-tauri/icons/source/small_win.svg";
import { TabBar } from "./TabBar";
import { WindowControls } from "./WindowControls";
import { drawsMenuBar } from "../lib/window-frame";

/** Whether the window has the system's focus, kept up to date */
function useWindowFocused(): boolean {
  const [focused, setFocused] = useState(() => document.hasFocus());
  useEffect(() => {
    const on = () => setFocused(true);
    const off = () => setFocused(false);
    window.addEventListener("focus", on);
    window.addEventListener("blur", off);
    return () => {
      window.removeEventListener("focus", on);
      window.removeEventListener("blur", off);
    };
  }, []);
  return focused;
}

/**
 * Whether the window fills its own space on macOS, where the traffic
 * lights hide, kept up to date
 */
function useFullScreen(enabled: boolean): boolean {
  const [fullScreen, setFullScreen] = useState(false);
  useEffect(() => {
    if (!enabled) return;
    const win = getCurrentWindow();
    const check = () => void win.isFullscreen().then(setFullScreen, () => {});
    check();
    const unlisten = win.onResized(check);
    return () => {
      void unlisten.then((fn) => fn(), () => {});
    };
  }, [enabled]);
  return fullScreen;
}

/**
 * The title bar the page draws (lib/window-frame.ts), with the tabs in it.
 * On Windows it holds the app's icon and the window's buttons too; on
 * macOS it leaves room at the left for the traffic lights, which the
 * system draws over it.  Its empty space moves the window, and a double
 * click there zooms it.
 */
export function TitleBar() {
  const focused = useWindowFocused();
  const windows = drawsMenuBar();
  const fullScreen = useFullScreen(!windows);
  const classes = [
    "title-bar",
    windows ? "is-windows" : "is-macos",
    focused ? "" : "is-inactive",
    fullScreen ? "is-full-screen" : "",
  ].filter(Boolean);
  return (
    <div className={classes.join(" ")} data-tauri-drag-region>
      {windows && <img className="title-bar-icon" src={appIcon} alt="" draggable={false} data-tauri-drag-region />}
      <TabBar />
      {windows && <WindowControls />}
    </div>
  );
}
