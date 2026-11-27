import { useEffect, useState } from "react";

/**
 * A text field that saves when it loses focus or on Enter.  Esc puts back
 * the saved value, and closes nothing else while the draft differs.
 */
export function TextField({
  id,
  describedBy,
  value,
  placeholder,
  onSave,
  mono = false,
  readOnly = false,
  className,
}: {
  id?: string;
  describedBy?: string;
  value: string;
  placeholder?: string;
  onSave?: (value: string) => void;
  /** Monospace, for paths, addresses and tokens */
  mono?: boolean;
  readOnly?: boolean;
  className?: string;
}) {
  const [draft, setDraft] = useState(value);
  useEffect(() => setDraft(value), [value]);
  const save = () => {
    if (draft !== value) onSave?.(draft);
  };
  const classes = ["ui-text-field", mono && "ui-text-field--mono", className].filter(Boolean).join(" ");
  return (
    <input
      id={id}
      aria-describedby={describedBy}
      type="text"
      className={classes}
      placeholder={placeholder}
      value={draft}
      readOnly={readOnly}
      spellCheck={false}
      onChange={(e) => setDraft(e.target.value)}
      onBlur={save}
      onKeyDown={(e) => {
        if (e.key === "Enter") {
          save();
        } else if (e.key === "Escape" && draft !== value) {
          e.stopPropagation();
          setDraft(value);
        }
      }}
    />
  );
}
