import type { Meta, StoryObj } from "@storybook/react-vite";
import type { ReactNode } from "react";
import { DocsPanel } from "../components/DocsPanel";
import { LogWindow, StatusBar } from "../components/LogPanel";
import { useLogStore } from "../store/logStore";
import type { LogTab } from "../types/log";
import { seedNotebook } from "./store";
import { WaitFor } from "./wait";

const meta: Meta = { title: "Panels", tags: ["themes"] };
export default meta;

const noop = () => {};

/** The docs panel at the height it has beside a notebook */
function Docs({ name }: { name?: string }) {
  return (
    <div style={{ display: "flex", justifyContent: "flex-end", height: 640 }}>
      <DocsPanel open functionName={name} requestId={name ? 1 : 0} onClose={noop} />
      <WaitFor selector={name ? ".docs-function-name" : ".docs-panel"} />
    </div>
  );
}

/** A function's page: signatures, the manual's text with maths and code, examples, see also */
export const DocsFunction: StoryObj = {
  render: () => <Docs name="integrate" />,
};

/** A package function with no page of its own: its package and how to load it */
export const DocsPackageFunction: StoryObj = {
  render: () => <Docs name="pdf_normal" />,
};

/** The panel opened from the toolbar, before choosing a function */
export const DocsEmpty: StoryObj = {
  render: () => <Docs />,
};

const start = new Date(2026, 10, 27, 9, 30, 0).getTime();

/** The log as it is after a session that couldn't start the AI tools */
function seedLog(tab: LogTab) {
  const log = useLogStore.getState();
  log.clearLog();
  log.clearRawOutput();
  log.clearEnvelopes();
  log.addEntry("info", "Maxima 5.50.0 started", "session");
  log.addEntry("info", "MCP server listening on 127.0.0.1:19542", "mcp");
  log.addEntry("warning", "Evaluation took longer than 10 s", "session");
  log.addEntry("error", "Claude Code is installed but Aximar isn't configured in it", "mcp", { kind: "settings", section: "ai-assistants" });
  // Fixed times, so screenshots don't change from run to run
  useLogStore.setState((s) => ({ entries: s.entries.map((e, i) => ({ ...e, timestamp: start + i * 1500 })) }));
  const lines: [string, "stdin" | "stdout" | "stderr"][] = [
    ["integrate(1/(1+x^2), x);", "stdin"],
    ["(%o2)                              atan(x)", "stdout"],
    ["1/0;", "stdin"],
    ["expt: undefined: 0 to a negative exponent.", "stderr"],
  ];
  lines.forEach(([line, stream], i) => log.addRawOutput(line, stream, start + 4000 + i * 200));
  log.addEnvelope(start + 5000, '{"kind":"cell_started","cell":"c1"}', "cell_started", null);
  log.addEnvelope(start + 5100, '{"kind":"cell_finished","cell":"c1","ms":12}', "cell_finished", null);
  log.addEnvelope(start + 5200, "{not json", null, "expected property name");
  log.openWindow();
  log.setActiveTab(tab);
}

function Log(): ReactNode {
  return (
    <div style={{ display: "flex", flexDirection: "column", justifyContent: "flex-end", height: 320 }}>
      <LogWindow />
      <StatusBar />
      <WaitFor selector=".log-window-body [data-index]" />
    </div>
  );
}

function logStory(tab: LogTab): StoryObj {
  return {
    loaders: [
      async () => {
        seedNotebook();
        seedLog(tab);
      },
    ],
    render: () => <Log />,
  };
}

/** The app log, with an entry linking to the Settings section that fixes it */
export const LogApp = logStory("app");
/** What Aximar sent Maxima and what Maxima printed */
export const LogMaximaOutput = logStory("maxima");
/** The kernel's event stream, with a line that didn't parse */
export const LogEvents = logStory("events");

/** The status bar alone, with the latest entry and an unread count */
export const StatusBarUnread: StoryObj = {
  loaders: [
    async () => {
      seedLog("app");
      useLogStore.setState({ windowOpen: false, unreadCount: 3 });
    },
  ],
  render: () => <StatusBar />,
};
