/** A native checkbox in the accent colour */
export function Checkbox({
  id,
  describedBy,
  checked,
  onChange,
  title,
}: {
  id?: string;
  describedBy?: string;
  checked: boolean;
  onChange: (checked: boolean) => void;
  title?: string;
}) {
  return (
    <input
      id={id}
      aria-describedby={describedBy}
      type="checkbox"
      className="ui-checkbox"
      checked={checked}
      title={title}
      onChange={(e) => onChange(e.target.checked)}
    />
  );
}
