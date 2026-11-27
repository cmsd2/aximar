import { useRef, type KeyboardEvent } from "react";
import { useLingui } from "@lingui/react/macro";
import { labelText, type Label } from "./label";

/**
 * A choice of one value from a few, as joined buttons.  A radio group:
 * the arrow keys, Home and End move the choice.  Labels never wrap; a
 * group too wide for its space scrolls sideways.
 */
export function SegmentedControl<T extends string>({
  id,
  describedBy,
  options,
  value,
  onChange,
}: {
  id?: string;
  describedBy?: string;
  options: { value: T; label: Label }[];
  value: string;
  onChange: (value: T) => void;
}) {
  const { i18n } = useLingui();
  const buttons = useRef<(HTMLButtonElement | null)[]>([]);
  const current = Math.max(0, options.findIndex((o) => o.value === value));

  const onKeyDown = (e: KeyboardEvent) => {
    const last = options.length - 1;
    const to =
      e.key === "ArrowRight" || e.key === "ArrowDown" ? Math.min(current + 1, last)
      : e.key === "ArrowLeft" || e.key === "ArrowUp" ? Math.max(current - 1, 0)
      : e.key === "Home" ? 0
      : e.key === "End" ? last
      : null;
    if (to === null) return;
    e.preventDefault();
    onChange(options[to].value);
    buttons.current[to]?.focus();
  };

  return (
    <div className="ui-segmented" role="radiogroup" id={id} aria-describedby={describedBy} onKeyDown={onKeyDown}>
      {options.map((o, i) => (
        <button
          key={o.value}
          ref={(el) => {
            buttons.current[i] = el;
          }}
          type="button"
          role="radio"
          aria-checked={i === current && value === o.value}
          tabIndex={i === current ? 0 : -1}
          className="ui-segmented-option"
          onClick={() => onChange(o.value)}
        >
          {labelText(i18n, o.label)}
        </button>
      ))}
    </div>
  );
}
