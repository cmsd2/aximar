import { useEffect, useState } from "react";
import { useLingui } from "@lingui/react/macro";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { SystemChrome } from "../lib/system-chrome";
import type { TitleBarItem } from "../lib/window-frame";

// Segoe Fluent Icons' caption glyphs, which Windows 11 draws its own with
const MINIMIZE = "";
const MAXIMIZE = "";
const RESTORE = "";
const CLOSE = "";

/** Whether the window fills the screen, kept up to date */
export function useMaximized(): boolean {
  const [maximized, setMaximized] = useState(false);
  useEffect(() => {
    const win = getCurrentWindow();
    const check = () => void win.isMaximized().then(setMaximized, () => {});
    check();
    const unlisten = win.onResized(check);
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);
  return maximized;
}

/** The buttons' tooltips, the same on every platform */
function useLabels(maximized: boolean) {
  const { t } = useLingui();
  return {
    minimize: t({ message: "Minimize", comment: "Windows title bar: tooltip of the button that minimises the window to the taskbar" }),
    maximize: maximized
      ? t({ message: "Restore Down", comment: "Windows title bar: tooltip of the button that returns a maximised window to its size" })
      : t({ message: "Maximize", comment: "Windows title bar: tooltip of the button that makes the window fill the screen" }),
    close: t({ message: "Close", comment: "Windows title bar: tooltip of the button that closes the window" }),
  };
}

// A button takes no focus, so a click leaves the keyboard where it was
const keepFocus = (e: React.MouseEvent) => e.preventDefault();

/**
 * Minimise, maximise and close, at the title bar's right, for a window
 * whose page draws its title bar on Windows (lib/window-frame.ts)
 */
export function WindowControls() {
  const maximized = useMaximized();
  const label = useLabels(maximized);
  const win = getCurrentWindow();
  return (
    <div className="window-controls">
      <button className="window-control" title={label.minimize} aria-label={label.minimize} onMouseDown={keepFocus} onClick={() => void win.minimize()}>
        {MINIMIZE}
      </button>
      <button className="window-control" title={label.maximize} aria-label={label.maximize} onMouseDown={keepFocus} onClick={() => void win.toggleMaximize()}>
        {maximized ? RESTORE : MAXIMIZE}
      </button>
      <button className="window-control window-close" title={label.close} aria-label={label.close} onMouseDown={keepFocus} onClick={() => void win.close()}>
        {CLOSE}
      </button>
    </div>
  );
}

type Button = Exclude<TitleBarItem, "icon">;

/**
 * Each button's glyph on a 16px grid.  Breeze draws chevrons for minimise
 * and maximise and a diamond for restore; Adwaita and most other GTK
 * themes draw a bar, a square and two squares.
 */
function Glyph({ button, maximized, desktop }: { button: Button; maximized: boolean; desktop: SystemChrome["desktop"] }) {
  const kde = desktop === "kde";
  const path =
    button === "close"
      ? "M4 4l8 8M12 4l-8 8"
      : button === "minimize"
        ? kde
          ? "M3.5 6l4.5 4.5L12.5 6"
          : "M4 11.5h8"
        : maximized
          ? kde
            ? "M8 3.5L12.5 8 8 12.5 3.5 8z"
            : "M4 6h6v6H4zM6 6V4h6v6h-2"
          : kde
            ? "M3.5 10.5L8 6l4.5 4.5"
            : "M4 4h8v8H4z";
  return (
    <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" fill="none" stroke="currentColor" strokeWidth="1.25" strokeLinecap="round" strokeLinejoin="round">
      <path d={path} />
    </svg>
  );
}

/**
 * The window's buttons on Linux, in the order the desktop lays them out
 * (`gtk-decoration-layout`), drawn as Breeze draws them on KDE and as
 * Adwaita does elsewhere
 */
export function LinuxWindowControls({ buttons, desktop }: { buttons: Button[]; desktop: SystemChrome["desktop"] }) {
  const maximized = useMaximized();
  const label = useLabels(maximized);
  const win = getCurrentWindow();
  const run: Record<Button, () => void> = {
    minimize: () => void win.minimize(),
    maximize: () => void win.toggleMaximize(),
    close: () => void win.close(),
  };
  return (
    <div className={`window-controls linux-window-controls is-${desktop}`}>
      {buttons.map((b) => (
        <button
          key={b}
          className={`linux-window-control${b === "close" ? " window-close" : ""}`}
          title={label[b]}
          aria-label={label[b]}
          onMouseDown={keepFocus}
          onClick={run[b]}
        >
          <Glyph button={b} maximized={maximized} desktop={desktop} />
        </button>
      ))}
    </div>
  );
}
