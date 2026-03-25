import { useState, useEffect, useCallback, useRef } from "react";
import { getConfig, setConfig, listWslDistros, checkWslMaxima, markdownFontStack, applyPrintMargins, claudeMcpStatus, claudeMcpConfigure, type AppConfig, type ClaudeMcpStatus } from "../lib/config-client";
import { useNotebookStore, type Theme, type CellStyle, type AutocompleteMode } from "../store/notebookStore";
import { useLogStore } from "../store/logStore";

interface SettingsModalProps {
  onClose: () => void;
  onSetVariablesOpen: (open: boolean) => void;
}

const FONT_SIZES = [12, 13, 14, 15, 16];
const PRINT_FONT_SIZES = [8, 9, 10, 11, 12, 13, 14, 15, 16];
const EVAL_TIMEOUTS = [10, 30, 60, 120];
const THEMES: Theme[] = ["auto", "light", "dark"];
const CELL_STYLES: CellStyle[] = ["card", "bracket"];
const MARKDOWN_FONTS: { value: string; label: string }[] = [
  { value: "sans-serif", label: "Sans-serif" },
  { value: "serif", label: "Serif" },
  { value: "computer-modern", label: "Computer Modern" },
  { value: "mono", label: "Mono" },
];
const MARKDOWN_INDENTS: { value: string; label: string }[] = [
  { value: "flush", label: "Flush" },
  { value: "aligned", label: "Aligned" },
];
const AUTOCOMPLETE_MODES: { value: AutocompleteMode; label: string }[] = [
  { value: "hint", label: "Hint" },
  { value: "snippet", label: "Snippet" },
  { value: "active-hint", label: "Active hint" },
];
const BACKENDS: { value: string; label: string }[] = [
  { value: "local", label: "Local" },
  { value: "docker", label: "Docker" },
  { value: "wsl", label: "WSL" },
];
const CONTAINER_ENGINES: { value: string; label: string }[] = [
  { value: "docker", label: "Docker" },
  { value: "podman", label: "Podman" },
];

function McpTokenRow({ token, onRegenerate }: { token: string; onRegenerate: (t: string) => void }) {
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
    const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
    onRegenerate(hex);
  }, [onRegenerate]);

  return (
    <div className="settings-row">
      <label className="settings-label">MCP token</label>
      <div className="settings-control">
        <div className="settings-token-actions">
          <input
            type="text"
            className="settings-input settings-token-input"
            readOnly
            value={token}
          />
          <button className="settings-token-btn" onClick={handleCopy} title="Copy token">
            {copied ? "Copied" : "Copy"}
          </button>
          <button className="settings-token-btn" onClick={handleRegenerate} title="Generate new token (restarts MCP server)">
            Regenerate
          </button>
        </div>
      </div>
    </div>
  );
}

function ClaudeCodeSetup({ address, token }: { address: string; token: string }) {
  const [status, setStatus] = useState<ClaudeMcpStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const checkStatus = useCallback(() => {
    claudeMcpStatus()
      .then((s) => { setStatus(s); setError(null); })
      .catch(() => setStatus({ installed: false, configured: false }));
  }, []);

  useEffect(() => { checkStatus(); }, [checkStatus, address, token]);

  const handleConfigure = useCallback(() => {
    const url = `http://${address}/mcp`;
    setLoading(true);
    setError(null);
    claudeMcpConfigure(url, token)
      .then(() => { checkStatus(); })
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [address, token, checkStatus]);

  const notInstalled = status !== null && !status.installed;

  return (
    <div className="settings-row">
      <label className="settings-label">Claude Code</label>
      <div className="settings-control">
        <div className="settings-claude-setup">
          <button
            className="settings-token-btn"
            disabled={loading || notInstalled}
            onClick={handleConfigure}
          >
            {loading ? "Configuring..." : status?.configured ? "Reconfigure" : "Configure"}
          </button>
          {status === null ? null : notInstalled ? (
            <span className="settings-claude-status settings-claude-warn">claude CLI not found</span>
          ) : status.configured ? (
            <span className="settings-claude-status settings-claude-ok">Configured</span>
          ) : error ? (
            <span className="settings-claude-status settings-claude-err">{error}</span>
          ) : (
            <span className="settings-claude-status settings-claude-warn">Not configured</span>
          )}
        </div>
      </div>
    </div>
  );
}

export function SettingsModal({ onClose, onSetVariablesOpen }: SettingsModalProps) {
  const [config, setLocalConfig] = useState<AppConfig | null>(null);
  const [wslDistros, setWslDistros] = useState<string[]>([]);
  const [wslMaximaPath, setWslMaximaPath] = useState<string | null | undefined>(undefined);
  const setTheme = useNotebookStore((s) => s.setTheme);
  const setCellStyle = useNotebookStore((s) => s.setCellStyle);
  const setAutocompleteMode = useNotebookStore((s) => s.setAutocompleteMode);
  const addLogEntry = useLogStore((s) => s.addEntry);

  useEffect(() => {
    getConfig()
      .then((resp) => setLocalConfig(resp.config))
      .catch((e) => addLogEntry("error", `Failed to load config: ${e}`, "settings"));
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (config?.backend === "wsl") {
      listWslDistros()
        .then(setWslDistros)
        .catch((e) => {
          addLogEntry("error", `Failed to list WSL distros: ${e}`, "settings");
          setWslDistros([]);
        });
    }
  }, [config?.backend]); // eslint-disable-line react-hooks/exhaustive-deps

  useEffect(() => {
    if (config?.backend === "wsl") {
      setWslMaximaPath(undefined);
      checkWslMaxima(config.wsl_distro)
        .then(setWslMaximaPath)
        .catch((e) => {
          addLogEntry("error", `Failed to check WSL Maxima: ${e}`, "settings");
          setWslMaximaPath(null);
        });
    }
  }, [config?.backend, config?.wsl_distro]); // eslint-disable-line react-hooks/exhaustive-deps

  const update = useCallback(
    (updates: Partial<AppConfig>) => {
      if (!config) return;
      const next = { ...config, ...updates };
      setLocalConfig(next);
      setConfig(updates).catch((e) => addLogEntry("error", `Failed to save settings: ${e}`, "settings"));

      if (updates.theme) {
        setTheme(updates.theme as Theme);
      }
      if (updates.cell_style) {
        setCellStyle(updates.cell_style as CellStyle);
        document.documentElement.dataset.cellStyle = updates.cell_style;
      }
      if (updates.autocomplete_mode) {
        setAutocompleteMode(updates.autocomplete_mode as AutocompleteMode);
      }
      if (updates.markdown_font) {
        document.documentElement.style.setProperty(
          "--font-family-markdown",
          markdownFontStack(updates.markdown_font)
        );
      }
      if (updates.markdown_indent) {
        document.documentElement.style.setProperty(
          "--markdown-indent",
          updates.markdown_indent === "aligned" ? "var(--gutter-width)" : "16px"
        );
      }
      if (updates.variables_open !== undefined) {
        onSetVariablesOpen(updates.variables_open);
      }
      if (updates.font_size !== undefined) {
        document.documentElement.style.setProperty(
          "--font-size-mono",
          `${updates.font_size}px`
        );
      }
      if (updates.print_font_size !== undefined) {
        document.documentElement.style.setProperty(
          "--print-font-size",
          `${updates.print_font_size}px`
        );
        document.documentElement.style.setProperty(
          "--print-font-size-mono",
          `${updates.print_font_size - 1}px`
        );
      }
      if (
        updates.print_margin_top !== undefined ||
        updates.print_margin_bottom !== undefined ||
        updates.print_margin_left !== undefined ||
        updates.print_margin_right !== undefined
      ) {
        applyPrintMargins(
          updates.print_margin_top ?? next.print_margin_top,
          updates.print_margin_bottom ?? next.print_margin_bottom,
          updates.print_margin_left ?? next.print_margin_left,
          updates.print_margin_right ?? next.print_margin_right,
        );
      }
    },
    [config, setTheme, setCellStyle, setAutocompleteMode, onSetVariablesOpen, addLogEntry]
  );

  if (!config) return null;

  return (
    <div className="palette-overlay" onClick={onClose}>
      <div className="settings-modal" onClick={(e) => e.stopPropagation()}>
        <div className="settings-header">
          <h2 className="settings-title">Settings</h2>
        </div>
        <div className="settings-body">
          <div className="settings-section">
            <div className="settings-row">
              <label className="settings-label">Theme</label>
              <div className="settings-control">
                <div className="settings-theme-group">
                  {THEMES.map((t) => (
                    <button
                      key={t}
                      className={`settings-theme-btn${config.theme === t ? " active" : ""}`}
                      onClick={() => update({ theme: t })}
                    >
                      {t.charAt(0).toUpperCase() + t.slice(1)}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            <div className="settings-row">
              <label className="settings-label">Cell style</label>
              <div className="settings-control">
                <div className="settings-theme-group">
                  {CELL_STYLES.map((s) => (
                    <button
                      key={s}
                      className={`settings-theme-btn${config.cell_style === s ? " active" : ""}`}
                      onClick={() => update({ cell_style: s })}
                    >
                      {s.charAt(0).toUpperCase() + s.slice(1)}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            <div className="settings-row">
              <label className="settings-label">Markdown font</label>
              <div className="settings-control">
                <div className="settings-theme-group">
                  {MARKDOWN_FONTS.map((f) => (
                    <button
                      key={f.value}
                      className={`settings-theme-btn${config.markdown_font === f.value ? " active" : ""}`}
                      onClick={() => update({ markdown_font: f.value })}
                    >
                      {f.label}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            <div className="settings-row">
              <label className="settings-label">Markdown indent</label>
              <div className="settings-control">
                <div className="settings-theme-group">
                  {MARKDOWN_INDENTS.map((i) => (
                    <button
                      key={i.value}
                      className={`settings-theme-btn${config.markdown_indent === i.value ? " active" : ""}`}
                      onClick={() => update({ markdown_indent: i.value })}
                    >
                      {i.label}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            <div className="settings-row">
              <label className="settings-label">Argument help</label>
              <div className="settings-control">
                <div className="settings-theme-group">
                  {AUTOCOMPLETE_MODES.map((m) => (
                    <button
                      key={m.value}
                      className={`settings-theme-btn${config.autocomplete_mode === m.value ? " active" : ""}`}
                      onClick={() => update({ autocomplete_mode: m.value })}
                    >
                      {m.label}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            <div className="settings-row">
              <label className="settings-label">Backend</label>
              <div className="settings-control">
                <div className="settings-theme-group">
                  {BACKENDS.map((b) => (
                    <button
                      key={b.value}
                      className={`settings-theme-btn${config.backend === b.value ? " active" : ""}`}
                      onClick={() => update({ backend: b.value })}
                    >
                      {b.label}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            {config.backend === "docker" && (
              <>
                <div className="settings-row">
                  <label className="settings-label">Container engine</label>
                  <div className="settings-control">
                    <div className="settings-theme-group">
                      {CONTAINER_ENGINES.map((e) => (
                        <button
                          key={e.value}
                          className={`settings-theme-btn${config.container_engine === e.value ? " active" : ""}`}
                          onClick={() => update({ container_engine: e.value })}
                        >
                          {e.label}
                        </button>
                      ))}
                    </div>
                  </div>
                </div>

                <div className="settings-row">
                  <label className="settings-label">Docker image</label>
                  <div className="settings-control">
                    <input
                      type="text"
                      className="settings-input"
                      placeholder="e.g. aximar/maxima"
                      value={config.docker_image}
                      onChange={(e) =>
                        update({ docker_image: e.target.value })
                      }
                    />
                  </div>
                </div>
              </>
            )}

            {config.backend === "wsl" && (
              <div className="settings-row">
                <label className="settings-label">WSL distro</label>
                <div className="settings-control">
                  <select
                    className="settings-select"
                    value={config.wsl_distro}
                    onChange={(e) =>
                      update({ wsl_distro: e.target.value })
                    }
                  >
                    <option value="">Default</option>
                    {wslDistros.map((d) => (
                      <option key={d} value={d}>
                        {d}
                      </option>
                    ))}
                  </select>
                  <span
                    className="settings-wsl-status"
                    title={
                      wslMaximaPath === undefined
                        ? "Checking..."
                        : wslMaximaPath
                          ? `Found: ${wslMaximaPath}`
                          : "maxima not found in this distro"
                    }
                  >
                    {wslMaximaPath === undefined
                      ? ""
                      : wslMaximaPath
                        ? `maxima found`
                        : "maxima not found"}
                  </span>
                </div>
              </div>
            )}

            {config.backend === "local" && (
              <div className="settings-row">
                <label className="settings-label">Maxima path</label>
                <div className="settings-control">
                  <input
                    type="text"
                    className="settings-input"
                    placeholder="Auto-detect"
                    value={config.maxima_path ?? ""}
                    onChange={(e) =>
                      update({
                        maxima_path: e.target.value || null,
                      })
                    }
                  />
                </div>
              </div>
            )}

            <div className="settings-row">
              <label className="settings-label">Font size</label>
              <div className="settings-control">
                <select
                  className="settings-select"
                  value={config.font_size}
                  onChange={(e) =>
                    update({ font_size: Number(e.target.value) })
                  }
                >
                  {FONT_SIZES.map((s) => (
                    <option key={s} value={s}>
                      {s}px
                    </option>
                  ))}
                </select>
              </div>
            </div>

            <div className="settings-row">
              <label className="settings-label">Print font size</label>
              <div className="settings-control">
                <select
                  className="settings-select"
                  value={config.print_font_size}
                  onChange={(e) =>
                    update({ print_font_size: Number(e.target.value) })
                  }
                >
                  {PRINT_FONT_SIZES.map((s) => (
                    <option key={s} value={s}>
                      {s}px
                    </option>
                  ))}
                </select>
              </div>
            </div>

            <div className="settings-row">
              <label className="settings-label">Print margins (mm)</label>
              <div className="settings-control">
                <div className="settings-margins">
                  {(["top", "bottom", "left", "right"] as const).map((side) => {
                    const key = `print_margin_${side}` as keyof AppConfig;
                    return (
                      <label key={side} className="settings-margin-field">
                        <span className="settings-margin-label">{side.charAt(0).toUpperCase() + side.slice(1)}</span>
                        <input
                          type="number"
                          className="settings-margin-input"
                          min={0}
                          max={50}
                          value={config[key] as number}
                          onChange={(e) =>
                            update({ [key]: Math.max(0, Math.min(50, Number(e.target.value))) })
                          }
                        />
                      </label>
                    );
                  })}
                </div>
              </div>
            </div>

            <div className="settings-row">
              <label className="settings-label">Eval timeout</label>
              <div className="settings-control">
                <select
                  className="settings-select"
                  value={config.eval_timeout}
                  onChange={(e) =>
                    update({ eval_timeout: Number(e.target.value) })
                  }
                >
                  {EVAL_TIMEOUTS.map((t) => (
                    <option key={t} value={t}>
                      {t}s
                    </option>
                  ))}
                </select>
              </div>
            </div>

            <div className="settings-row">
              <label className="settings-label">MCP server</label>
              <div className="settings-control">
                <input
                  type="checkbox"
                  className="settings-checkbox"
                  checked={config.mcp_enabled}
                  onChange={(e) =>
                    update({ mcp_enabled: e.target.checked })
                  }
                />
              </div>
            </div>

            {config.mcp_enabled && (
              <>
                <div className="settings-row">
                  <label className="settings-label">MCP listen address</label>
                  <div className="settings-control">
                    <input
                      type="text"
                      className="settings-input"
                      placeholder="127.0.0.1:19542"
                      value={config.mcp_listen_address}
                      onBlur={(e) =>
                        update({ mcp_listen_address: e.target.value })
                      }
                      onChange={(e) =>
                        setLocalConfig({ ...config, mcp_listen_address: e.target.value })
                      }
                    />
                  </div>
                </div>

                <McpTokenRow token={config.mcp_token} onRegenerate={(newToken) => update({ mcp_token: newToken })} />

                <ClaudeCodeSetup address={config.mcp_listen_address} token={config.mcp_token} />
              </>
            )}

            <div className="settings-row">
              <label className="settings-label">Variable panel open by default</label>
              <div className="settings-control">
                <input
                  type="checkbox"
                  className="settings-checkbox"
                  checked={config.variables_open}
                  onChange={(e) =>
                    update({ variables_open: e.target.checked })
                  }
                />
              </div>
            </div>
          </div>
        </div>
        <div className="settings-footer">
          <button className="template-skip" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
