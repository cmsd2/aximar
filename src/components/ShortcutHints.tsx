import { useState, useEffect } from "react";
import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";

const isMac = navigator.platform.toUpperCase().includes("MAC");
const modSymbol = isMac ? "\u2318" : "Ctrl+";
const shiftSymbol = isMac ? "\u21E7" : "Shift+";

interface Hint {
  keys: string;
  label: MessageDescriptor;
}

const baseHints: Hint[] = [
  {
    keys: "Z",
    label: msg({ message: "Undo", comment: "Shortcut hints shown while Cmd or Ctrl is held: undoes the last edit. max-length:16" }),
  },
  {
    keys: `${shiftSymbol}Z`,
    label: msg({ message: "Redo", comment: "Shortcut hints shown while Cmd or Ctrl is held: redoes the last undone edit. max-length:16" }),
  },
  {
    keys: "F",
    label: msg({ message: "Find", comment: "Shortcut hints shown while Cmd or Ctrl is held: opens the find bar. max-length:16" }),
  },
  {
    keys: `${shiftSymbol}F`,
    label: msg({ message: "Find & Replace", comment: "Shortcut hints shown while Cmd or Ctrl is held: opens find and replace. max-length:16" }),
  },
  {
    keys: "D",
    label: msg({ message: "Delete Cell", comment: "Shortcut hints shown while Cmd or Ctrl is held: deletes the active cell. max-length:16" }),
  },
  {
    keys: `${shiftSymbol}\u2191`,
    label: msg({ message: "Move Up", comment: "Shortcut hints shown while Cmd or Ctrl is held: moves the active cell up one place. max-length:16" }),
  },
  {
    keys: `${shiftSymbol}\u2193`,
    label: msg({ message: "Move Down", comment: "Shortcut hints shown while Cmd or Ctrl is held: moves the active cell down one place. max-length:16" }),
  },
  {
    keys: "K",
    label: msg({ message: "Palette", comment: "Shortcut hints shown while Cmd or Ctrl is held: opens the command palette. max-length:16" }),
  },
];

/** The hints, shown while Cmd or Ctrl is held, or always with `held` (for the gallery) */
export function ShortcutHints({ held = false }: { held?: boolean }) {
  const { i18n } = useLingui();
  const [pressed, setPressed] = useState(false);
  const visible = held || pressed;

  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Meta" || e.key === "Control") {
        setPressed(true);
      }
    };
    const onKeyUp = (e: KeyboardEvent) => {
      if (e.key === "Meta" || e.key === "Control") {
        setPressed(false);
      }
    };
    const onBlur = () => setPressed(false);

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
          <span>{i18n._(h.label)}</span>
        </span>
      ))}
    </div>
  );
}
