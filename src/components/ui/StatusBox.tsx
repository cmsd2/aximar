import type { ReactNode } from "react";
import { IconAlertTriangle, IconCircleCheck, IconInfoCircle, IconLoader2 } from "./icons";

export type StatusTone = "ok" | "warn" | "err" | "busy" | "info";

const ICONS = {
  ok: IconCircleCheck,
  warn: IconAlertTriangle,
  err: IconAlertTriangle,
  busy: IconLoader2,
  info: IconInfoCircle,
};

/**
 * A tinted box with an icon and one or two lines of text, for the state
 * of something: Maxima found, a server listening, a check that failed
 */
export function StatusBox({
  tone,
  children,
  detail,
  status,
}: {
  tone: StatusTone;
  children: ReactNode;
  /** A second, quieter line */
  detail?: ReactNode;
  /** A name for tests and styles, as data-status */
  status?: string;
}) {
  const Icon = ICONS[tone];
  return (
    <div className={`ui-status ui-status--${tone}`} role="status" data-status={status}>
      <Icon className="ui-status-icon" size={16} stroke={1.75} aria-hidden="true" />
      <div className="ui-status-text">
        <span className="ui-status-main">{children}</span>
        {detail && <span className="ui-status-detail">{detail}</span>}
      </div>
    </div>
  );
}
