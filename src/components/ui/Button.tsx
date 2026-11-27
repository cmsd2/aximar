import type { ButtonHTMLAttributes, ReactNode } from "react";
import type { Icon } from "./icons";

type Common = Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children"> & {
  /** primary for the one main action, secondary for others, quiet for minor ones */
  variant?: "primary" | "secondary" | "quiet";
  size?: "sm" | "md";
};

/** With text, and an optional icon before it */
type WithText = Common & { children: ReactNode; icon?: Icon; label?: string };

/** An icon alone: `label`, already translated, names it for screen readers and the tooltip */
type IconOnly = Common & { children?: undefined; icon: Icon; label: string };

export type ButtonProps = WithText | IconOnly;

/** A button: the one way the window draws one */
export function Button({ variant = "secondary", size = "md", icon: Icon, label, children, className, ...rest }: ButtonProps) {
  const iconOnly = children === undefined;
  const classes = [
    "ui-button",
    `ui-button--${variant}`,
    iconOnly ? "ui-button--icon" : `ui-button--${size}`,
    className,
  ]
    .filter(Boolean)
    .join(" ");
  return (
    <button
      type="button"
      className={classes}
      aria-label={iconOnly ? label : undefined}
      title={label}
      {...rest}
    >
      {Icon && <Icon size={16} stroke={1.75} aria-hidden="true" />}
      {children}
    </button>
  );
}
