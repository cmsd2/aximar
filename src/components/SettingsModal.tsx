import { useState, useEffect, useCallback } from "react";
import { getConfig, setConfig, type AppConfig } from "../lib/config-client";
import { useNotebookStore, type Theme, type CellStyle } from "../store/notebookStore";

interface SettingsModalProps {
  onClose: () => void;
  onSetVariablesOpen: (open: boolean) => void;
}

const FONT_SIZES = [12, 13, 14, 15, 16];
const EVAL_TIMEOUTS = [10, 30, 60, 120];
const THEMES: Theme[] = ["auto", "light", "dark"];
const CELL_STYLES: CellStyle[] = ["card", "bracket"];

export function SettingsModal({ onClose, onSetVariablesOpen }: SettingsModalProps) {
  const [config, setLocalConfig] = useState<AppConfig | null>(null);
  const setTheme = useNotebookStore((s) => s.setTheme);
  const setCellStyle = useNotebookStore((s) => s.setCellStyle);

  useEffect(() => {
    getConfig().then((resp) => setLocalConfig(resp.config)).catch(() => {});
  }, []);

  const update = useCallback(
    (updates: Partial<AppConfig>) => {
      if (!config) return;
      const next = { ...config, ...updates };
      setLocalConfig(next);
      setConfig(updates).catch(() => {});

      if (updates.theme) {
        setTheme(updates.theme as Theme);
      }
      if (updates.cell_style) {
        setCellStyle(updates.cell_style as CellStyle);
        document.documentElement.dataset.cellStyle = updates.cell_style;
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
    },
    [config, setTheme, setCellStyle, onSetVariablesOpen]
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
