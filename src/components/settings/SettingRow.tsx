import { useEffect, useId, useState, type ReactNode } from "react";
import type { I18n, MessageDescriptor } from "@lingui/core";
import { useLingui } from "@lingui/react/macro";
import type { AppConfig } from "../../lib/config-client";

/** What every section gets: the config, and a way to change it */
export interface SectionProps {
  config: AppConfig;
  /** Change settings: applied and saved at once */
  update: (updates: Partial<AppConfig>) => void;
}

/** A button label: a message to translate, or a product name kept as it is */
export type Label = MessageDescriptor | string;

export function labelText(i18n: I18n, label: Label): string {
  return typeof label === "string" ? label : i18n._(label);
}

/**
 * One setting: its label and a one-line hint on the left, its control on
 * the right.  The control gets `id` for the label and `aria-describedby`
 * for the hint through `children`'s arguments.
 */
export function SettingRow({
  label,
  hint,
  children,
  className,
}: {
  label: ReactNode;
  hint: ReactNode;
  children: (ids: { id: string; describedBy: string }) => ReactNode;
  className?: string;
}) {
  const id = useId();
  const hintId = `${id}-hint`;
  return (
    <div className={`settings-row${className ? ` ${className}` : ""}`}>
      <div className="settings-text">
        <label className="settings-label" htmlFor={id}>
          {label}
        </label>
        <p className="settings-hint" id={hintId}>
          {hint}
        </p>
      </div>
      <div className="settings-control">{children({ id, describedBy: hintId })}</div>
    </div>
  );
}

/** A group of buttons choosing one value */
export function ChoiceButtons<T extends string>({
  id,
  describedBy,
  options,
  value,
  onChange,
}: {
  id: string;
  describedBy: string;
  options: { value: T; label: Label }[];
  value: string;
  onChange: (value: T) => void;
}) {
  const { i18n } = useLingui();
  return (
    <div className="settings-theme-group" role="radiogroup" id={id} aria-describedby={describedBy}>
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          role="radio"
          aria-checked={value === o.value}
          className={`settings-theme-btn${value === o.value ? " active" : ""}`}
          onClick={() => onChange(o.value)}
        >
          {labelText(i18n, o.label)}
        </button>
      ))}
    </div>
  );
}

/**
 * A text field that saves when it loses focus or on Enter.  Esc puts back
 * the saved value, without closing Settings.
 */
export function TextSetting({
  id,
  describedBy,
  value,
  placeholder,
  onSave,
  className,
}: {
  id: string;
  describedBy: string;
  value: string;
  placeholder?: string;
  onSave: (value: string) => void;
  className?: string;
}) {
  const [draft, setDraft] = useState(value);
  useEffect(() => setDraft(value), [value]);
  const save = () => {
    if (draft !== value) onSave(draft);
  };
  return (
    <input
      id={id}
      aria-describedby={describedBy}
      type="text"
      className={`settings-input${className ? ` ${className}` : ""}`}
      placeholder={placeholder}
      value={draft}
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
