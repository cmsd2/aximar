import { useCallback, useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { mcpServerStatus, type McpServerState } from "../../lib/config-client";
import { StatusBox } from "../ui/StatusBox";
import { useLingui } from "@lingui/react/macro";
import { AiClients } from "./AiClients";
import { Button } from "../ui/Button";
import { Checkbox } from "../ui/Checkbox";
import { FieldRow } from "../ui/FieldRow";
import { TextField } from "../ui/TextField";
import type { SectionProps } from "./types";

export function AiAssistantsSection({ config, update }: SectionProps) {
  const { t } = useLingui();
  return (
    <>
      <FieldRow
        divided
        label={t({
          message: "MCP server",
          comment: "Settings: the label of the checkbox that turns on Aximar's MCP server for AI tools",
        })}
        hint={t({
          message: "Lets AI assistants read and run your notebooks.",
          comment: "Settings, AI assistants section: hint under MCP server",
        })}
      >
        {({ id, describedBy }) => (
          <Checkbox id={id} describedBy={describedBy} checked={config.mcp_enabled} onChange={(checked) => update({ mcp_enabled: checked })} />
        )}
      </FieldRow>

      {config.mcp_enabled && <McpServerStatusLine />}

      {config.mcp_enabled && (
        <>
          <AiClients address={config.mcp_listen_address} token={config.mcp_token} />

          <details className="settings-advanced">
            <summary>
              {t({
                message: "Advanced",
                comment: "Settings, AI assistants section: the heading that shows the MCP server's address and token when opened",
              })}
            </summary>
            <div className="settings-section">
            <FieldRow
        divided
              label={t({
                message: "MCP listen address",
                comment: "Settings: the label of the field for the address and port the MCP server listens on",
              })}
              hint={t({
                message: "The address and port AI assistants connect to.",
                comment: "Settings, AI assistants section: hint under MCP listen address",
              })}
            >
              {(ids) => (
                <TextField
                  {...ids}
                  mono
                  value={config.mcp_listen_address}
                  placeholder="127.0.0.1:19542"
                  onSave={(mcp_listen_address) => update({ mcp_listen_address })}
                />
              )}
            </FieldRow>
            <McpTokenRow token={config.mcp_token} onRegenerate={(mcp_token) => update({ mcp_token })} />
            </div>
          </details>
        </>
      )}
    </>
  );
}

function McpTokenRow({ token, onRegenerate }: { token: string; onRegenerate: (token: string) => void }) {
  const { t } = useLingui();
  const [copied, setCopied] = useState(false);
  const timerRef = useRef<ReturnType<typeof setTimeout>>(undefined);

  const handleCopy = useCallback(() => {
    navigator.clipboard.writeText(token).then(() => {
      setCopied(true);
      clearTimeout(timerRef.current);
      timerRef.current = setTimeout(() => setCopied(false), 2000);
    });
  }, [token]);

  const handleRegenerate = useCallback(() => {
    const bytes = new Uint8Array(32);
    crypto.getRandomValues(bytes);
    onRegenerate(Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join(""));
  }, [onRegenerate]);

  return (
    <FieldRow
        divided
      wide
      label={t({
        message: "MCP token",
        comment: "Settings: the label of the secret that AI tools send to Aximar's MCP server",
      })}
      hint={t({
        message: "AI assistants send this secret with each request. Reconfigure them after a change.",
        comment: "Settings, AI assistants section: hint under MCP token. Reconfigure is the button beside each AI assistant",
      })}
    >
      {({ id, describedBy }) => (
        <div className="settings-token-actions">
          <TextField id={id} describedBy={describedBy} mono readOnly value={token} className="settings-token-input" />
          <Button
            size="sm"
            onClick={handleCopy}
            title={t({ message: "Copy token", comment: "Settings: tooltip of the button that copies the MCP token" })}
          >
            {copied
              ? t({ message: "Copied", comment: "Settings: the MCP token copy button, for two seconds after a copy" })
              : t({ message: "Copy", comment: "Settings button: copies the MCP token to the clipboard" })}
          </Button>
          <Button
            size="sm"
            onClick={handleRegenerate}
            title={t({
              message: "Generate new token (restarts MCP server)",
              comment: "Settings: tooltip of the Regenerate button for the MCP token",
            })}
          >
            {t({ message: "Regenerate", comment: "Settings button: replaces the MCP token with a new random one" })}
          </Button>
        </div>
      )}
    </FieldRow>
  );
}

/** Whether the MCP server is listening, and why not if it failed */
function McpServerStatusLine() {
  const { t } = useLingui();
  const [status, setStatus] = useState<McpServerState | null>(null);

  useEffect(() => {
    let current = true;
    mcpServerStatus()
      .then((s) => current && setStatus(s))
      .catch(() => {});
    const unlisten = listen<McpServerState>("mcp-server-status", (e) => current && setStatus(e.payload));
    return () => {
      current = false;
      void unlisten.then((f) => f());
    };
  }, []);

  if (!status) return null;
  switch (status.state) {
    case "listening": {
      const address = status.address;
      return (
        <StatusBox tone="ok" status="mcp-server">
          {t({
            message: `Listening on ${address}.`,
            comment: "Settings, AI assistants section: the MCP server's status. address is its address and port, such as 127.0.0.1:19542",
          })}
        </StatusBox>
      );
    }
    case "failed": {
      const { address, reason } = status;
      return (
        <StatusBox tone="err" status="mcp-server" detail={reason}>
          {t({
            message: `Couldn't listen on ${address}.`,
            comment: "Settings, AI assistants section: the MCP server's status when it failed to start; the system's reason follows. address is its address and port",
          })}
        </StatusBox>
      );
    }
    default:
      return (
        <StatusBox tone="busy" status="mcp-server">
          {t({ message: "Starting...", comment: "Settings, AI assistants section: the MCP server's status while it starts" })}
        </StatusBox>
      );
  }
}
