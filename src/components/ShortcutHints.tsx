import { useState, useEffect } from "react";

const isMac = navigator.platform.toUpperCase().includes("MAC");
const modSymbol = isMac ? "\u2318" : "Ctrl+";
const shiftSymbol = isMac ? "\u21E7" : "Shift+";

interface Hint {
  keys: string;
  label: string;
}

const baseHints: Hint[] = [
  { keys: "Z", label: "Undo" },
  { keys: `${shiftSymbol}Z`, label: "Redo" },
  { keys: "F", label: "Find" },
  { keys: `${shiftSymbol}F`, label: "Find & Replace" },
  { keys: "D", label: "Delete Cell" },
  { keys: `${shiftSymbol}\u2191`, label: "Move Up" },
  { keys: `${shiftSymbol}\u2193`, label: "Move Down" },
  { keys: "K", label: "Palette" },
];

export function ShortcutHints() {
  const [visible, setVisible] = useState(false);

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Meta" || e.key === "Control") {
        setVisible(true);
      }
    };
    const onKeyUp = (e: KeyboardEvent) => {
      if (e.key === "Meta" || e.key === "Control") {
        setVisible(false);
      }
    };
    const onBlur = () => setVisible(false);

    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("keyup", onKeyUp);
    window.addEventListener("blur", onBlur);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("keyup", onKeyUp);
      window.removeEventListener("blur", onBlur);
    };
  }, []);

  return (
    <div className="shortcut-hints">
      {visible && baseHints.map((h) => (
        <span key={h.keys} className="shortcut-hint">
          <kbd>{modSymbol}{h.keys}</kbd>
          <span>{h.label}</span>
        </span>
      ))}
    </div>
  );
}
