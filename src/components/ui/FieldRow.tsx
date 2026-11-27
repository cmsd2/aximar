import { useId, type ReactNode } from "react";

/**
 * One field: its label and a one-line hint on the left, its control on
 * the right.  The control gets `id` for the label and `describedBy` for
 * the hint through `children`'s arguments.
 */
export function FieldRow({
  label,
  hint,
  children,
  divided = false,
  wide = false,
}: {
  label: ReactNode;
  hint?: ReactNode;
  children: (ids: { id: string; describedBy?: string }) => ReactNode;
  /** A line under the row, for lists of fields such as Settings */
  divided?: boolean;
  /** The control under the text, full width, for long controls */
  wide?: boolean;
}) {
  const id = useId();
  const hintId = hint ? `${id}-hint` : undefined;
  const classes = ["ui-field-row", divided && "ui-field-row--divided", wide && "ui-field-row--wide"]
    .filter(Boolean)
    .join(" ");
  return (
    <div className={classes}>
      <div className="ui-field-text">
        <label className="ui-field-label" htmlFor={id}>
          {label}
        </label>
        {hint && (
          <p className="ui-field-hint" id={hintId}>
            {hint}
          </p>
        )}
      </div>
      <div className="ui-field-control">{children({ id, describedBy: hintId })}</div>
    </div>
  );
}
