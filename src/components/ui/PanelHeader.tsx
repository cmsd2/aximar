import type { ReactNode } from "react";
import { Button } from "./Button";
import { IconX } from "./icons";

/**
 * The one header the panels share (variables, docs, log, the find bar):
 * a title or controls on the left, actions and a close button on the right.
 * `closeLabel`, already translated, names the close button.
 */
export function PanelHeader({
  title,
  children,
  actions,
  onClose,
  closeLabel,
  className,
}: {
  title?: ReactNode;
  children?: ReactNode;
  actions?: ReactNode;
  onClose?: () => void;
  closeLabel?: string;
  className?: string;
}) {
  return (
    <div className={["ui-panel-header", className].filter(Boolean).join(" ")}>
      <div className="ui-panel-header-main">
        {title !== undefined && <span className="ui-panel-header-title">{title}</span>}
        {children}
      </div>
      {(actions || onClose) && (
        <div className="ui-panel-header-actions">
          {actions}
          {onClose && <Button variant="quiet" icon={IconX} label={closeLabel ?? ""} className="ui-panel-close" onClick={onClose} />}
        </div>
      )}
    </div>
  );
}
