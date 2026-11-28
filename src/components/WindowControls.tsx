import { useEffect, useState } from "react";
import { useLingui } from "@lingui/react/macro";
import { getCurrentWindow } from "@tauri-apps/api/window";

// Segoe Fluent Icons' caption glyphs, which Windows 11 draws its own with
const MINIMIZE = "";
const MAXIMIZE = "";
const RESTORE = "";
const CLOSE = "";

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

/**
 * Minimise, maximise and close, at the title bar's right, for a window
 * whose page draws its title bar (lib/window-frame.ts)
 */
export function WindowControls() {
  const { t } = useLingui();
  const maximized = useMaximized();
  const win = getCurrentWindow();
  const minimizeLabel = t({ message: "Minimize", comment: "Windows title bar: tooltip of the button that minimises the window to the taskbar" });
  const maximizeLabel = maximized
    ? t({ message: "Restore Down", comment: "Windows title bar: tooltip of the button that returns a maximised window to its size" })
    : t({ message: "Maximize", comment: "Windows title bar: tooltip of the button that makes the window fill the screen" });
  const closeLabel = t({ message: "Close", comment: "Windows title bar: tooltip of the button that closes the window" });
  // A button takes no focus, so a click leaves the keyboard where it was
  const keepFocus = (e: React.MouseEvent) => e.preventDefault();
  return (
    <div className="window-controls">
      <button className="window-control" title={minimizeLabel} aria-label={minimizeLabel} onMouseDown={keepFocus} onClick={() => void win.minimize()}>
        {MINIMIZE}
      </button>
      <button className="window-control" title={maximizeLabel} aria-label={maximizeLabel} onMouseDown={keepFocus} onClick={() => void win.toggleMaximize()}>
        {maximized ? RESTORE : MAXIMIZE}
      </button>
      <button className="window-control window-close" title={closeLabel} aria-label={closeLabel} onMouseDown={keepFocus} onClick={() => void win.close()}>
        {CLOSE}
      </button>
    </div>
  );
}
