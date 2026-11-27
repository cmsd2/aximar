import { useCallback, useRef, useState } from "react";
import { useLingui } from "@lingui/react/macro";
import { AiClients } from "./AiClients";
import { SettingRow, TextSetting, type SectionProps } from "./SettingRow";

export function AiAssistantsSection({ config, update }: SectionProps) {
  const { t } = useLingui();
  return (
    <>
      <SettingRow
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
          <input
            id={id}
            aria-describedby={describedBy}
            type="checkbox"
            className="settings-checkbox"
            checked={config.mcp_enabled}
            onChange={(e) => update({ mcp_enabled: e.target.checked })}
          />
        )}
      </SettingRow>

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
            <SettingRow
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
                <TextSetting
                  {...ids}
                  value={config.mcp_listen_address}
                  placeholder="127.0.0.1:19542"
                  onSave={(mcp_listen_address) => update({ mcp_listen_address })}
                />
              )}
            </SettingRow>
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
    <SettingRow
      className="settings-row-wide"
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
          <input id={id} aria-describedby={describedBy} type="text" className="settings-input settings-token-input" readOnly value={token} />
          <button
            type="button"
            className="settings-token-btn"
            onClick={handleCopy}
            title={t({ message: "Copy token", comment: "Settings: tooltip of the button that copies the MCP token" })}
          >
            {copied
              ? t({ message: "Copied", comment: "Settings: the MCP token copy button, for two seconds after a copy" })
              : t({ message: "Copy", comment: "Settings button: copies the MCP token to the clipboard" })}
          </button>
          <button
            type="button"
            className="settings-token-btn"
            onClick={handleRegenerate}
            title={t({
              message: "Generate new token (restarts MCP server)",
              comment: "Settings: tooltip of the Regenerate button for the MCP token",
            })}
          >
            {t({ message: "Regenerate", comment: "Settings button: replaces the MCP token with a new random one" })}
          </button>
        </div>
      )}
    </SettingRow>
  );
}
