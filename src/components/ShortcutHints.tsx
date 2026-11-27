import { useState, useEffect } from "react";
import { useLingui } from "@lingui/react/macro";
import { COMMANDS } from "../commands/registry";
import { formatShortcut } from "../commands/shortcut";

/** The commands marked as hints, with their shortcuts */
const hints = COMMANDS.filter((c) => c.hint && c.shortcut);

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
      {visible && hints.map((c) => (
        <span key={c.id} className="shortcut-hint">
          <kbd>{formatShortcut(c.shortcut!)}</kbd>
          <span>{i18n._(c.label)}</span>
        </span>
      ))}
    </div>
  );
}
