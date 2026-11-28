import { useEffect, useState } from "react";
import appIcon from "../../src-tauri/icons/source/small_win.svg";
import { TabBar } from "./TabBar";
import { WindowControls } from "./WindowControls";

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
 * The title bar the page draws on Windows (lib/window-frame.ts): the app's
 * icon, the tabs, and the window's buttons.  Its empty space moves the
 * window, and a double click there maximises it.
 */
export function TitleBar() {
  const focused = useWindowFocused();
  return (
    <div className={`title-bar${focused ? "" : " is-inactive"}`} data-tauri-drag-region>
      <img className="title-bar-icon" src={appIcon} alt="" draggable={false} data-tauri-drag-region />
      <TabBar />
      <WindowControls />
    </div>
  );
}
