import { useLingui } from "@lingui/react/macro";
import { useActiveTab } from "../store/notebookStore";
import { useLogStore } from "../store/logStore";
import { command, isEnabled, type CommandState } from "../commands/registry";
import { formatShortcut } from "../commands/shortcut";
import { useCommandState, useRunCommand } from "../commands/useCommands";
import { Button } from "./ui/Button";
import {
  IconBook,
  IconListDetails,
  IconPlayerStop,
  IconPlayerTrackNext,
  IconRefresh,
  IconSettings,
  IconVariable,
  type Icon,
} from "./ui/icons";

/**
 * The toolbar: the session's commands beside its status on the left, the
 * panels and Settings on the right.  Each button is a command from the
 * command list, with its label and shortcut as the tooltip.
 */
export function Toolbar() {
  const { t } = useLingui();
  const state = useCommandState();
  const sessionStatus = useActiveTab().sessionStatus;
  const unread = useLogStore((s) => s.unreadCount);
  const logOpen = useLogStore((s) => s.windowOpen);

  let statusText: string;
  if (sessionStatus === "Starting") {
    statusText = t({ message: "Starting", comment: "Toolbar: session status while Maxima starts up. max-length:12" });
  } else if (sessionStatus === "Ready") {
    statusText = t({ message: "Ready", comment: "Toolbar: session status when Maxima is ready to run cells. max-length:12" });
  } else if (sessionStatus === "Busy") {
    statusText = t({ message: "Busy", comment: "Toolbar: session status while Maxima runs a cell. max-length:12" });
  } else if (sessionStatus === "Stopped") {
    statusText = t({ message: "Stopped", comment: "Toolbar: session status when Maxima is not running. max-length:12" });
  } else {
    const message = sessionStatus.Error;
    statusText = t({
      message: `Error: ${message}`,
      comment: "Toolbar: session status when Maxima failed. message is the error from the backend, in English",
    });
  }

  const statusClass =
    sessionStatus === "Ready"
      ? "status-ready"
      : sessionStatus === "Starting"
        ? "status-starting"
        : "status-error";

  return (
    <div className="toolbar">
      <div className="toolbar-left">
        <CommandButton id="cell.run_all" icon={IconPlayerTrackNext} state={state} />
        <CommandButton id="session.interrupt" icon={IconPlayerStop} state={state} />
        <CommandButton id="session.restart" icon={IconRefresh} state={state} />
        <span className={`session-status ${statusClass}`}>{statusText}</span>
      </div>
      <div className="toolbar-right">
        <CommandButton id="view.variables" icon={IconVariable} state={state} />
        <CommandButton id="view.docs" icon={IconBook} state={state} />
        <span className="toolbar-btn-wrapper">
          <CommandButton id="view.log" icon={IconListDetails} state={state} />
          {!logOpen && unread > 0 && <span className="log-badge">{unread}</span>}
        </span>
        <div className="toolbar-separator" />
        <CommandButton id="app.settings" icon={IconSettings} state={state} />
      </div>
    </div>
  );
}

/** An icon button that runs a command; a toggle, pressed while its panel is open, for a checked command */
function CommandButton({ id, icon, state }: { id: string; icon: Icon; state: CommandState }) {
  const { i18n } = useLingui();
  const run = useRunCommand();
  const c = command(id);
  if (!c) return null;
  const label = i18n._(c.label);
  const tooltip = c.shortcut ? `${label} (${formatShortcut(c.shortcut)})` : label;
  const pressed = c.checked ? c.checked(state) : undefined;
  return (
    <Button
      variant="quiet"
      icon={icon}
      label={label}
      title={tooltip}
      className={pressed === undefined ? "toolbar-icon" : `toolbar-icon toolbar-toggle${pressed ? " is-on" : ""}`}
      aria-pressed={pressed}
      disabled={!isEnabled(c, state)}
      onClick={() => void run(id)}
    />
  );
}
