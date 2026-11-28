import { useEffect, useMemo, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import appIcon from "../../src-tauri/icons/source/small_win.svg";
import { TabBar } from "./TabBar";
import { LinuxWindowControls, WindowControls } from "./WindowControls";
import { currentPlatform } from "../commands/shortcut";
import { useSystemChromeStore } from "../lib/system-chrome";
import { parseDecorationLayout, type TitleBarItem } from "../lib/window-frame";

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

const icon = <img key="icon" className="title-bar-icon" src={appIcon} alt="" draggable={false} data-tauri-drag-region />;

/** One end of a Linux title bar: the icon and the buttons, in the desktop's order */
function LinuxEnd({ items, side }: { items: TitleBarItem[]; side: "start" | "end" }) {
  const desktop = useSystemChromeStore((s) => s.chrome?.desktop ?? "other");
  const out: React.ReactNode[] = [];
  let run: Exclude<TitleBarItem, "icon">[] = [];
  const flush = () => {
    if (run.length) out.push(<LinuxWindowControls key={`buttons-${out.length}`} buttons={run} desktop={desktop} />);
    run = [];
  };
  for (const item of items) {
    if (item === "icon") {
      flush();
      out.push(icon);
    } else run.push(item);
  }
  flush();
  return (
    <div className={`title-bar-end is-${side}`} data-tauri-drag-region>
      {out}
    </div>
  );
}

/**
 * The title bar the page draws (lib/window-frame.ts), with the tabs in it.
 * On Windows it holds the app's icon and the window's buttons too; on
 * Linux the icon and the buttons go where the desktop puts them; on macOS
 * it leaves room at the left for the traffic lights, which the system
 * draws over it.  Its empty space moves the window, and a double click
 * there zooms it.
 */
export function TitleBar() {
  const focused = useWindowFocused();
  const platform = currentPlatform();
  const fullScreen = useFullScreen(platform === "macos");
  const decorationLayout = useSystemChromeStore((s) => s.chrome?.decorationLayout);
  const layout = useMemo(() => parseDecorationLayout(decorationLayout), [decorationLayout]);
  const classes = [
    "title-bar",
    `is-${platform}`,
    focused ? "" : "is-inactive",
    fullScreen ? "is-full-screen" : "",
  ].filter(Boolean);
  return (
    <div className={classes.join(" ")} data-tauri-drag-region>
      {platform === "windows" && icon}
      {platform === "linux" && <LinuxEnd items={layout.start} side="start" />}
      <TabBar />
      {platform === "windows" && <WindowControls />}
      {platform === "linux" && <LinuxEnd items={layout.end} side="end" />}
    </div>
  );
}
