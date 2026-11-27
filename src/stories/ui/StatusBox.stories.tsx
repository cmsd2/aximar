import type { Meta, StoryObj } from "@storybook/react-vite";
import { useLingui } from "@lingui/react/macro";
import { StatusBox } from "../../components/ui/StatusBox";

const meta: Meta = { title: "UI/StatusBox", tags: ["themes"] };
export default meta;

function Tones() {
  const { t } = useLingui();
  const path = "/opt/homebrew/bin/maxima";
  const version = "5.50.0";
  const address = "127.0.0.1:19542";
  return (
    <div style={{ maxWidth: 520 }}>
      <StatusBox
        tone="ok"
        detail={t({
          message: "In a standard install location.",
          comment: "Settings, Maxima section: under the found Maxima, where Aximar found it",
        })}
      >
        {t({
          message: `Found Maxima ${version} at ${path}`,
          comment: "Settings, Maxima section: status. version is Maxima's version, such as 5.50.0; path is the program",
        })}
      </StatusBox>
      <StatusBox tone="busy">
        {t({ message: "Starting...", comment: "Settings, AI assistants section: the MCP server's status while it starts" })}
      </StatusBox>
      <StatusBox tone="err" detail="Address already in use (os error 48)">
        {t({
          message: `Couldn't listen on ${address}.`,
          comment: "Settings, AI assistants section: the MCP server's status when it failed to start; the system's reason follows. address is its address and port",
        })}
      </StatusBox>
      <StatusBox tone="warn">{t({ message: "The check timed out", comment: "Settings: shown beside Claude Code when its claude command took too long to answer" })}</StatusBox>
      <StatusBox tone="info">{t({ message: "Changes apply as you make them.", comment: "Settings: a note at the bottom of each section; there is no Save button" })}</StatusBox>
    </div>
  );
}

/** Each tone: ok, busy, err with a second line, warn, info */
export const AllTones: StoryObj = { render: () => <Tones /> };
