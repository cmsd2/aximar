import { useCallback, useEffect, useState } from "react";
import { useLingui } from "@lingui/react/macro";
import {
  claudeMcpCheck,
  mcpClientsStatus,
  mcpConfigure,
  type ConfigState,
  type McpClient,
  type McpClientsStatus,
} from "../../lib/config-client";

const CLIENTS: { id: McpClient; name: string; cli: string }[] = [
  { id: "claude", name: "Claude Code", cli: "claude" },
  { id: "codex", name: "Codex", cli: "codex" },
  { id: "gemini", name: "Gemini CLI", cli: "gemini" },
];

/**
 * A row per AI client, with its Configure button and status.  Loading the
 * rows runs none of the clients' programs: Claude Code's status needs its
 * CLI, so it waits for Check.
 */
export function AiClients({ address, token }: { address: string; token: string }) {
  const [status, setStatus] = useState<McpClientsStatus | null>(null);

  const refresh = useCallback(async () => {
    try {
      const next = await mcpClientsStatus();
      // Keep a Claude Code result the user already asked for
      setStatus((old) =>
        old && old.claude.configured.state !== "unchecked"
          ? { ...next, claude: { ...next.claude, configured: old.claude.configured } }
          : next,
      );
    } catch {
      setStatus(null);
    }
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  const setClaude = (configured: ConfigState) =>
    setStatus((old) => (old ? { ...old, claude: { ...old.claude, configured } } : old));

  return (
    <>
      {CLIENTS.map((client) => (
        <AiClientRow
          key={client.id}
          client={client}
          installed={status ? status[client.id].installed : undefined}
          configured={status?.[client.id].configured ?? null}
          address={address}
          token={token}
          onConfigured={async () => {
            if (client.id === "claude") setClaude(await claudeMcpCheck());
            else await refresh();
          }}
          onCheck={client.id === "claude" ? async () => setClaude(await claudeMcpCheck()) : undefined}
        />
      ))}
    </>
  );
}

function AiClientRow({
  client,
  installed,
  configured,
  address,
  token,
  onConfigured,
  onCheck,
}: {
  client: { id: McpClient; name: string; cli: string };
  /** The command's path; null if not installed; undefined while loading */
  installed: string | null | undefined;
  configured: ConfigState | null;
  address: string;
  token: string;
  onConfigured: () => Promise<void>;
  onCheck?: () => Promise<void>;
}) {
  const { t } = useLingui();
  const [busy, setBusy] = useState<"configure" | "check" | null>(null);
  const [error, setError] = useState<string | null>(null);
  const notInstalled = installed === null;
  const cli = client.cli;

  const run = async (kind: "configure" | "check", action: () => Promise<void>) => {
    setBusy(kind);
    setError(null);
    try {
      await action();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  };

  const configure = () =>
    run("configure", async () => {
      await mcpConfigure(client.id, `http://${address}/mcp`, token);
      await onConfigured();
    });

  const yes = configured?.state === "yes";

  return (
    <div className="settings-row" data-client={client.id}>
      <label className="settings-label" title={installed ?? undefined}>
        {client.name}
      </label>
      <div className="settings-control">
        <div className="settings-claude-setup">
          <button className="settings-token-btn" disabled={busy !== null || notInstalled} onClick={() => void configure()}>
            {busy === "configure"
              ? t({ message: "Configuring...", comment: "Settings: the Configure button of an AI tool while it adds Aximar's MCP server" })
              : yes
                ? t({
                    message: "Reconfigure",
                    comment: "Settings button: adds Aximar's MCP server to an AI tool's configuration again, with the current address and token",
                  })
                : t({
                    message: "Configure",
                    comment: "Settings button: adds Aximar's MCP server to an AI tool's configuration, such as Claude Code's",
                  })}
          </button>
          {onCheck && !notInstalled && (
            <button className="settings-token-btn" disabled={busy !== null} onClick={() => void run("check", onCheck)}>
              {busy === "check"
                ? t({ message: "Checking...", comment: "Settings: the Check button of Claude Code while Aximar asks Claude Code whether it lists Aximar's MCP server" })
                : t({
                    message: "Check",
                    comment: "Settings button: asks Claude Code, by running its claude command, whether its configuration lists Aximar's MCP server",
                  })}
            </button>
          )}
          {error ? (
            <span className="settings-claude-status settings-claude-err">{error}</span>
          ) : installed === undefined ? null : notInstalled ? (
            <span className="settings-claude-status settings-claude-warn">
              {t({
                message: `${cli} CLI not found`,
                comment: "Settings: shown beside an AI tool when its command is not installed. cli is the command name, such as claude",
              })}
            </span>
          ) : (
            <ConfiguredStatus configured={configured} />
          )}
        </div>
      </div>
    </div>
  );
}

function ConfiguredStatus({ configured }: { configured: ConfigState | null }) {
  const { t } = useLingui();
  switch (configured?.state) {
    case undefined:
      return null;
    case "yes":
      return (
        <span className="settings-claude-status settings-claude-ok">
          {t({ message: "Configured", comment: "Settings: shown beside an AI tool whose configuration includes Aximar's MCP server" })}
        </span>
      );
    case "no":
      return (
        <span className="settings-claude-status settings-claude-warn">
          {t({
            message: "Not configured",
            comment: "Settings: shown beside an AI tool whose configuration does not include Aximar's MCP server",
          })}
        </span>
      );
    case "unchecked":
      return (
        <span className="settings-claude-status">
          {t({
            message: "Not checked",
            comment: "Settings: shown beside Claude Code until the user presses Check; Aximar doesn't run claude without being asked",
          })}
        </span>
      );
    case "timed-out":
      return (
        <span className="settings-claude-status settings-claude-err">
          {t({ message: "The check timed out", comment: "Settings: shown beside Claude Code when its claude command took too long to answer" })}
        </span>
      );
    case "error": {
      const file = configured.file;
      return (
        <span className="settings-claude-status settings-claude-err" title={configured.detail}>
          {t({
            message: `Can't read ${file}`,
            comment: "Settings: shown beside an AI tool whose configuration file isn't valid. file is the file's path; Aximar leaves the file alone",
          })}
        </span>
      );
    }
  }
}
