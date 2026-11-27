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
    <div className={`status-box ${tone}`} role="status" data-status={status}>
      <Icon className="status-box-icon" size={16} stroke={1.75} aria-hidden="true" />
      <div className="status-box-text">
        <span className="status-box-main">{children}</span>
        {detail && <span className="status-box-detail">{detail}</span>}
      </div>
    </div>
  );
}
