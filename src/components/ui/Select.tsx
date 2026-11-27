import type { ReactNode } from "react";

/** A native menu in the window's style */
export function Select<T extends string | number>({
  id,
  describedBy,
  value,
  onChange,
  options,
}: {
  id?: string;
  describedBy?: string;
  value: T;
  onChange: (value: T) => void;
  options: { value: T; label: ReactNode }[];
}) {
  return (
    <select
      id={id}
      aria-describedby={describedBy}
      className="ui-select"
      value={value}
      onChange={(e) => {
        const chosen = options.find((o) => String(o.value) === e.target.value);
        if (chosen) onChange(chosen.value);
      }}
    >
      {options.map((o) => (
        <option key={String(o.value)} value={o.value}>
          {o.label}
        </option>
      ))}
    </select>
  );
}
