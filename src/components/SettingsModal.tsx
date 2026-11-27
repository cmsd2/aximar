import { useState, useEffect, useCallback, useRef } from "react";
import type { I18n, MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import { activateLanguage, languageName } from "../lib/i18n";
import { getConfig, setConfig, listWslDistros, checkWslMaxima, markdownFontStack, applyPrintMargins, claudeMcpStatus, claudeMcpConfigure, codexMcpStatus, codexMcpConfigure, geminiMcpStatus, geminiMcpConfigure, type AppConfig, type ClaudeMcpStatus, type CodexMcpStatus, type GeminiMcpStatus } from "../lib/config-client";
import { useNotebookStore, type Theme, type CellStyle, type AutocompleteMode } from "../store/notebookStore";
import { useLogStore } from "../store/logStore";

interface SettingsModalProps {
  onClose: () => void;
  onSetVariablesOpen: (open: boolean) => void;
}

const FONT_SIZES = [12, 13, 14, 15, 16];
const PRINT_FONT_SIZES = [8, 9, 10, 11, 12, 13, 14, 15, 16];
const EVAL_TIMEOUTS = [10, 30, 60, 120];
/** A button label: a message to translate, or a product name kept as it is */
type Label = MessageDescriptor | string;

function labelText(i18n: I18n, label: Label): string {
  return typeof label === "string" ? label : i18n._(label);
}

const THEMES: { value: Theme; label: Label }[] = [
  {
    value: "auto",
    label: msg({ message: "Auto", comment: "Settings: theme button that follows the system's light or dark mode" }),
  },
  { value: "light", label: msg({ message: "Light", comment: "Settings: theme button for the light theme" }) },
  { value: "dark", label: msg({ message: "Dark", comment: "Settings: theme button for the dark theme" }) },
];
const CELL_STYLES: { value: CellStyle; label: Label }[] = [
  {
    value: "card",
    label: msg({ message: "Card", comment: "Settings: cell style button that draws each cell as a boxed card" }),
  },
  {
    value: "bracket",
    label: msg({ message: "Bracket", comment: "Settings: cell style button that marks each cell with a bracket in the margin" }),
  },
];
const MARKDOWN_FONTS: { value: string; label: Label }[] = [
  {
    value: "sans-serif",
    label: msg({ message: "Sans-serif", comment: "Settings: Markdown font button for the system sans-serif font" }),
  },
  { value: "serif", label: msg({ message: "Serif", comment: "Settings: Markdown font button for the system serif font" }) },
  { value: "computer-modern", label: "Computer Modern" },
  { value: "mono", label: msg({ message: "Mono", comment: "Settings: Markdown font button for the monospace font" }) },
];
const MARKDOWN_INDENTS: { value: string; label: Label }[] = [
  {
    value: "flush",
    label: msg({ message: "Flush", comment: "Settings: Markdown indent button; text cells start at the left edge" }),
  },
  {
    value: "aligned",
    label: msg({ message: "Aligned", comment: "Settings: Markdown indent button; text cells line up with the code in code cells" }),
  },
];
const AUTOCOMPLETE_MODES: { value: AutocompleteMode; label: Label }[] = [
  {
    value: "hint",
    label: msg({ message: "Hint", comment: "Settings: argument help button; shows a function's arguments as a hint while typing" }),
  },
  {
    value: "snippet",
    label: msg({ message: "Snippet", comment: "Settings: argument help button; completing a function inserts its arguments as placeholders to fill in" }),
  },
  {
    value: "active-hint",
    label: msg({ message: "Active hint", comment: "Settings: argument help button; the hint follows the argument under the cursor" }),
  },
];
const BACKENDS: { value: string; label: Label }[] = [
  {
    value: "local",
    label: msg({ message: "Local", comment: "Settings: backend button; runs Maxima installed on this computer" }),
  },
  { value: "docker", label: "Docker" },
  { value: "wsl", label: "WSL" },
];
const CONTAINER_ENGINES: { value: string; label: Label }[] = [
  { value: "docker", label: "Docker" },
  { value: "podman", label: "Podman" },
];
const MARGIN_SIDES: { side: "top" | "bottom" | "left" | "right"; label: Label }[] = [
  { side: "top", label: msg({ message: "Top", comment: "Settings: label of the top print margin field" }) },
  { side: "bottom", label: msg({ message: "Bottom", comment: "Settings: label of the bottom print margin field" }) },
  { side: "left", label: msg({ message: "Left", comment: "Settings: label of the left print margin field" }) },
  { side: "right", label: msg({ message: "Right", comment: "Settings: label of the right print margin field" }) },
];

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
    const hex = Array.from(bytes, (b) => b.toString(16).padStart(2, "0")).join("");
    onRegenerate(hex);
  }, [onRegenerate]);

  return (
    <div className="settings-row">
      <label className="settings-label">
        {t({
          message: "MCP token",
          comment: "Settings: the label of the secret that AI tools send to Aximar's MCP server",
        })}
      </label>
      <div className="settings-control">
        <div className="settings-token-actions">
          <input
            type="text"
            className="settings-input settings-token-input"
            readOnly
            value={token}
          />
          <button
            className="settings-token-btn"
            onClick={handleCopy}
            title={t({ message: "Copy token", comment: "Settings: tooltip of the button that copies the MCP token" })}
          >
            {copied
              ? t({ message: "Copied", comment: "Settings: the MCP token copy button, for two seconds after a copy" })
              : t({ message: "Copy", comment: "Settings button: copies the MCP token to the clipboard" })}
          </button>
          <button
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
      </div>
    </div>
  );
}

/** The Configure button and status of one AI tool's MCP setup */
function McpSetupControls({
  cli,
  status,
  loading,
  error,
  onConfigure,
}: {
  /** The tool's command name, such as claude */
  cli: string;
  status: { installed: boolean; configured: boolean } | null;
  loading: boolean;
  error: string | null;
  onConfigure: () => void;
}) {
  const { t } = useLingui();
  const notInstalled = status !== null && !status.installed;

  return (
    <div className="settings-claude-setup">
      <button
        className="settings-token-btn"
        disabled={loading || notInstalled}
        onClick={onConfigure}
      >
        {loading
          ? t({ message: "Configuring...", comment: "Settings: the Configure button of an AI tool while it adds Aximar's MCP server" })
          : status?.configured
            ? t({
                message: "Reconfigure",
                comment: "Settings button: adds Aximar's MCP server to an AI tool's configuration again, with the current address and token",
              })
            : t({
                message: "Configure",
                comment: "Settings button: adds Aximar's MCP server to an AI tool's configuration, such as Claude Code's",
              })}
      </button>
      {status === null ? null : notInstalled ? (
        <span className="settings-claude-status settings-claude-warn">
          {t({
            message: `${cli} CLI not found`,
            comment: "Settings: shown beside an AI tool when its command is not installed. cli is the command name, such as claude",
          })}
        </span>
      ) : status.configured ? (
        <span className="settings-claude-status settings-claude-ok">
          {t({ message: "Configured", comment: "Settings: shown beside an AI tool whose configuration includes Aximar's MCP server" })}
        </span>
      ) : error ? (
        <span className="settings-claude-status settings-claude-err">{error}</span>
      ) : (
        <span className="settings-claude-status settings-claude-warn">
          {t({
            message: "Not configured",
            comment: "Settings: shown beside an AI tool whose configuration does not include Aximar's MCP server",
          })}
        </span>
      )}
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

  return (
    <div className="settings-row">
      <label className="settings-label">Claude Code</label>
      <div className="settings-control">
        <McpSetupControls
          cli="claude"
          status={status}
          loading={loading}
          error={error}
          onConfigure={handleConfigure}
        />
      </div>
    </div>
  );
}

function CodexSetup({ address, token }: { address: string; token: string }) {
  const [status, setStatus] = useState<CodexMcpStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const checkStatus = useCallback(() => {
    codexMcpStatus()
      .then((s) => { setStatus(s); setError(null); })
      .catch(() => setStatus({ installed: false, configured: false }));
  }, []);

  useEffect(() => { checkStatus(); }, [checkStatus, address, token]);

  const handleConfigure = useCallback(() => {
    const url = `http://${address}/mcp`;
    setLoading(true);
    setError(null);
    codexMcpConfigure(url, token)
      .then(() => { checkStatus(); })
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [address, token, checkStatus]);

  return (
    <div className="settings-row">
      <label className="settings-label">Codex</label>
      <div className="settings-control">
        <McpSetupControls
          cli="codex"
          status={status}
          loading={loading}
          error={error}
          onConfigure={handleConfigure}
        />
      </div>
    </div>
  );
}

function GeminiSetup({ address, token }: { address: string; token: string }) {
  const [status, setStatus] = useState<GeminiMcpStatus | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const checkStatus = useCallback(() => {
    geminiMcpStatus()
      .then((s) => { setStatus(s); setError(null); })
      .catch(() => setStatus({ installed: false, configured: false }));
  }, []);

  useEffect(() => { checkStatus(); }, [checkStatus, address, token]);

  const handleConfigure = useCallback(() => {
    const url = `http://${address}/mcp`;
    setLoading(true);
    setError(null);
    geminiMcpConfigure(url, token)
      .then(() => { checkStatus(); })
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
  }, [address, token, checkStatus]);

  return (
    <div className="settings-row">
      <label className="settings-label">Gemini CLI</label>
      <div className="settings-control">
        <McpSetupControls
          cli="gemini"
          status={status}
          loading={loading}
          error={error}
          onConfigure={handleConfigure}
        />
      </div>
    </div>
  );
}

/** The language of the window, the menus and Maxima's error messages */
function LanguageRow({
  language,
  languages,
  onChange,
}: {
  language: string;
  languages: string[];
  onChange: (language: string) => void;
}) {
  const { t } = useLingui();
  return (
    <div className="settings-row">
      <label className="settings-label">
        {t({ message: "Language", comment: "Settings: the label of the language picker" })}
      </label>
      <div className="settings-control">
        <select
          className="settings-select"
          value={language}
          onChange={(e) => onChange(e.target.value)}
        >
          {languages.map((code) => (
            <option key={code} value={code}>
              {languageName(code)}
            </option>
          ))}
        </select>
      </div>
    </div>
  );
}

export function SettingsModal({ onClose, onSetVariablesOpen }: SettingsModalProps) {
  const { t, i18n } = useLingui();
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

  const exampleImage = "aximar/maxima";

  return (
    <div className="palette-overlay" onClick={onClose}>
      <div className="settings-modal" onClick={(e) => e.stopPropagation()}>
        <div className="settings-header">
          <h2 className="settings-title">
            {t({ message: "Settings", comment: "Settings: the title of the settings window" })}
          </h2>
        </div>
        <div className="settings-body">
          <div className="settings-section">
            <LanguageRow
              language={config.language}
              languages={config.languages}
              onChange={(language) => {
                update({ language });
                void activateLanguage(language);
              }}
            />

            <div className="settings-row">
              <label className="settings-label">
                {t({ message: "Theme", comment: "Settings: the label of the theme buttons" })}
              </label>
              <div className="settings-control">
                <div className="settings-theme-group">
                  {THEMES.map((theme) => (
                    <button
                      key={theme.value}
                      className={`settings-theme-btn${config.theme === theme.value ? " active" : ""}`}
                      onClick={() => update({ theme: theme.value })}
                    >
                      {labelText(i18n, theme.label)}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            <div className="settings-row">
              <label className="settings-label">
                {t({ message: "Cell style", comment: "Settings: the label of the buttons that choose how cells are drawn" })}
              </label>
              <div className="settings-control">
                <div className="settings-theme-group">
                  {CELL_STYLES.map((s) => (
                    <button
                      key={s.value}
                      className={`settings-theme-btn${config.cell_style === s.value ? " active" : ""}`}
                      onClick={() => update({ cell_style: s.value })}
                    >
                      {labelText(i18n, s.label)}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            <div className="settings-row">
              <label className="settings-label">
                {t({ message: "Markdown font", comment: "Settings: the label of the buttons that choose the font of text cells" })}
              </label>
              <div className="settings-control">
                <div className="settings-theme-group">
                  {MARKDOWN_FONTS.map((f) => (
                    <button
                      key={f.value}
                      className={`settings-theme-btn${config.markdown_font === f.value ? " active" : ""}`}
                      onClick={() => update({ markdown_font: f.value })}
                    >
                      {labelText(i18n, f.label)}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            <div className="settings-row">
              <label className="settings-label">
                {t({ message: "Markdown indent", comment: "Settings: the label of the buttons that choose how far text cells are indented" })}
              </label>
              <div className="settings-control">
                <div className="settings-theme-group">
                  {MARKDOWN_INDENTS.map((i) => (
                    <button
                      key={i.value}
                      className={`settings-theme-btn${config.markdown_indent === i.value ? " active" : ""}`}
                      onClick={() => update({ markdown_indent: i.value })}
                    >
                      {labelText(i18n, i.label)}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            <div className="settings-row">
              <label className="settings-label">
                {t({ message: "Argument help", comment: "Settings: the label of the buttons that choose how function arguments are shown while typing" })}
              </label>
              <div className="settings-control">
                <div className="settings-theme-group">
                  {AUTOCOMPLETE_MODES.map((m) => (
                    <button
                      key={m.value}
                      className={`settings-theme-btn${config.autocomplete_mode === m.value ? " active" : ""}`}
                      onClick={() => update({ autocomplete_mode: m.value })}
                    >
                      {labelText(i18n, m.label)}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            <div className="settings-row">
              <label className="settings-label">
                {t({ message: "Backend", comment: "Settings: the label of the buttons that choose where Maxima runs" })}
              </label>
              <div className="settings-control">
                <div className="settings-theme-group">
                  {BACKENDS.map((b) => (
                    <button
                      key={b.value}
                      className={`settings-theme-btn${config.backend === b.value ? " active" : ""}`}
                      onClick={() => update({ backend: b.value })}
                    >
                      {labelText(i18n, b.label)}
                    </button>
                  ))}
                </div>
              </div>
            </div>

            {config.backend === "docker" && (
              <>
                <div className="settings-row">
                  <label className="settings-label">
                    {t({ message: "Container engine", comment: "Settings: the label of the buttons that choose the program that runs the Maxima container" })}
                  </label>
                  <div className="settings-control">
                    <div className="settings-theme-group">
                      {CONTAINER_ENGINES.map((e) => (
                        <button
                          key={e.value}
                          className={`settings-theme-btn${config.container_engine === e.value ? " active" : ""}`}
                          onClick={() => update({ container_engine: e.value })}
                        >
                          {labelText(i18n, e.label)}
                        </button>
                      ))}
                    </div>
                  </div>
                </div>

                <div className="settings-row">
                  <label className="settings-label">
                    {t({ message: "Docker image", comment: "Settings: the label of the field for the container image that holds Maxima" })}
                  </label>
                  <div className="settings-control">
                    <input
                      type="text"
                      className="settings-input"
                      placeholder={t({
                        message: `e.g. ${exampleImage}`,
                        comment: "Settings: placeholder in the Docker image field. exampleImage is an image name, such as aximar/maxima",
                      })}
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
                <label className="settings-label">
                  {t({ message: "WSL distro", comment: "Settings: the label of the menu that chooses the Linux distribution under WSL that runs Maxima" })}
                </label>
                <div className="settings-control">
                  <select
                    className="settings-select"
                    value={config.wsl_distro}
                    onChange={(e) =>
                      update({ wsl_distro: e.target.value })
                    }
                  >
                    <option value="">
                      {t({ message: "Default", comment: "Settings: WSL distro menu item for the distribution WSL uses by default" })}
                    </option>
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
                        ? t({
                            message: "Checking...",
                            comment: "Settings: tooltip beside the WSL distro menu while Aximar looks for maxima in it",
                          })
                        : wslMaximaPath
                          ? t({
                              message: `Found: ${wslMaximaPath}`,
                              comment: "Settings: tooltip beside the WSL distro menu. wslMaximaPath is where the maxima program is in the distro",
                            })
                          : t({
                              message: "maxima not found in this distro",
                              comment: "Settings: tooltip beside the WSL distro menu when the distro has no maxima program",
                            })
                    }
                  >
                    {wslMaximaPath === undefined
                      ? ""
                      : wslMaximaPath
                        ? t({
                            message: "maxima found",
                            comment: "Settings: shown beside the WSL distro menu when the distro has the maxima program",
                          })
                        : t({
                            message: "maxima not found",
                            comment: "Settings: shown beside the WSL distro menu when the distro has no maxima program",
                          })}
                  </span>
                </div>
              </div>
            )}

            {config.backend === "local" && (
              <div className="settings-row">
                <label className="settings-label">
                  {t({ message: "Maxima path", comment: "Settings: the label of the field for the location of the maxima program" })}
                </label>
                <div className="settings-control">
                  <input
                    type="text"
                    className="settings-input"
                    placeholder={t({
                      message: "Auto-detect",
                      comment: "Settings: placeholder in the Maxima path field; when empty, Aximar finds maxima itself",
                    })}
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
              <label className="settings-label">
                {t({ message: "Font size", comment: "Settings: the label of the menu that chooses the size of code text" })}
              </label>
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
              <label className="settings-label">
                {t({ message: "Print font size", comment: "Settings: the label of the menu that chooses the size of text when printing" })}
              </label>
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
              <label className="settings-label">
                {t({ message: "Print margins (mm)", comment: "Settings: the label of the four print margin fields, in millimetres" })}
              </label>
              <div className="settings-control">
                <div className="settings-margins">
                  {MARGIN_SIDES.map(({ side, label }) => {
                    const key = `print_margin_${side}` as keyof AppConfig;
                    return (
                      <label key={side} className="settings-margin-field">
                        <span className="settings-margin-label">{labelText(i18n, label)}</span>
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
              <label className="settings-label">
                {t({ message: "Eval timeout", comment: "Settings: the label of the menu that chooses how long a cell may run before Aximar stops it" })}
              </label>
              <div className="settings-control">
                <select
                  className="settings-select"
                  value={config.eval_timeout}
                  onChange={(e) =>
                    update({ eval_timeout: Number(e.target.value) })
                  }
                >
                  {EVAL_TIMEOUTS.map((secs) => (
                    <option key={secs} value={secs}>
                      {secs}s
                    </option>
                  ))}
                </select>
              </div>
            </div>

            <div className="settings-row">
              <label className="settings-label">
                {t({ message: "MCP server", comment: "Settings: the label of the checkbox that turns on Aximar's MCP server for AI tools" })}
              </label>
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
                  <label className="settings-label">
                    {t({ message: "MCP listen address", comment: "Settings: the label of the field for the address and port the MCP server listens on" })}
                  </label>
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
                <CodexSetup address={config.mcp_listen_address} token={config.mcp_token} />
                <GeminiSetup address={config.mcp_listen_address} token={config.mcp_token} />
              </>
            )}

            <div className="settings-row">
              <label className="settings-label">
                {t({ message: "Variable panel open by default", comment: "Settings: the label of the checkbox that opens the variable panel when Aximar starts" })}
              </label>
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
            {t({ message: "Close", comment: "Settings button: closes the settings window" })}
          </button>
        </div>
      </div>
    </div>
  );
}
