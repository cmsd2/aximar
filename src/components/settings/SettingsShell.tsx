import { useCallback, useEffect, useRef, useState, type KeyboardEvent } from "react";
import { useLingui } from "@lingui/react/macro";
import { IconX } from "@tabler/icons-react";
import { getConfig, setConfig, type AppConfig } from "../../lib/config-client";
import { applyConfig } from "../../lib/apply-config";
import { useNotebookStore } from "../../store/notebookStore";
import { useLogStore } from "../../store/logStore";
import { availableSections, sectionFor, type SectionId } from "./sections";

export interface SettingsProps {
  onClose: () => void;
  onSetVariablesOpen: (open: boolean) => void;
  /** The section to open at; the one shown last if not given */
  initialSection?: SectionId;
  /** Changes with each request to open Settings, so a request while open moves to its section */
  request?: number;
}

/** The Settings window: a sidebar of sections, and the chosen section's settings */
export function SettingsShell({ onClose, onSetVariablesOpen, initialSection, request }: SettingsProps) {
  const { t, i18n } = useLingui();
  const [config, setLocalConfig] = useState<AppConfig | null>(null);
  const [section, setSection] = useState<SectionId | null>(null);
  const setTheme = useNotebookStore((s) => s.setTheme);
  const setCellStyle = useNotebookStore((s) => s.setCellStyle);
  const setAutocompleteMode = useNotebookStore((s) => s.setAutocompleteMode);
  const addLogEntry = useLogStore((s) => s.addEntry);
  const tabs = useRef(new Map<SectionId, HTMLButtonElement>());

  useEffect(() => {
    getConfig()
      .then((resp) => {
        setLocalConfig(resp.config);
        const shown = sectionFor(initialSection ?? resp.config.settings_section);
        setSection(shown);
        if (initialSection && initialSection !== resp.config.settings_section) {
          void setConfig({ settings_section: shown }).catch(() => {});
        }
      })
      .catch((e) => addLogEntry("error", `Failed to load config: ${e}`, "settings"));
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  const update = useCallback(
    (updates: Partial<AppConfig>) => {
      if (!config) return;
      const next = { ...config, ...updates };
      setLocalConfig(next);
      setConfig(updates).catch((e) => addLogEntry("error", `Failed to save settings: ${e}`, "settings"));
      applyConfig(updates, next, { setTheme, setCellStyle, setAutocompleteMode, setVariablesOpen: onSetVariablesOpen });
    },
    [config, setTheme, setCellStyle, setAutocompleteMode, onSetVariablesOpen, addLogEntry],
  );

  const sections = availableSections();
  const loaded = config !== null;

  // Asked to open at a section while already open
  const firstRequest = useRef(request);
  useEffect(() => {
    if (!loaded || request === firstRequest.current) return;
    firstRequest.current = request;
    if (initialSection) choose(sectionFor(initialSection), true);
  }, [request, loaded]); // eslint-disable-line react-hooks/exhaustive-deps

  const choose = (id: SectionId, focus = false) => {
    setSection(id);
    if (config && config.settings_section !== id) update({ settings_section: id });
    if (focus) tabs.current.get(id)?.focus();
  };

  const onSidebarKey = (e: KeyboardEvent) => {
    const at = sections.findIndex((s) => s.id === section);
    const to =
      e.key === "ArrowDown" ? Math.min(at + 1, sections.length - 1)
      : e.key === "ArrowUp" ? Math.max(at - 1, 0)
      : e.key === "Home" ? 0
      : e.key === "End" ? sections.length - 1
      : null;
    if (to === null) return;
    e.preventDefault();
    choose(sections[to].id, true);
  };

  const onKey = (e: KeyboardEvent) => {
    if (e.key !== "Escape" || e.defaultPrevented) return;
    // Esc in a menu closes the menu
    if ((e.target as Element).tagName === "SELECT") return;
    e.stopPropagation();
    onClose();
  };

  if (!config || !section) return null;
  const current = sections.find((s) => s.id === section) ?? sections[0];
  const Page = current.Component;

  return (
    <div className="palette-overlay" onClick={onClose}>
      <div
        className="settings-window"
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-title"
        onClick={(e) => e.stopPropagation()}
        onKeyDown={onKey}
      >
        <nav className="settings-sidebar">
          <h2 className="settings-sidebar-title" id="settings-title">
            {t({ message: "Settings", comment: "Settings: the title of the settings window" })}
          </h2>
          <div
            className="settings-tabs"
            role="tablist"
            aria-orientation="vertical"
            aria-labelledby="settings-title"
            onKeyDown={onSidebarKey}
          >
          {sections.map((s) => {
            const Icon = s.icon;
            return (
              <button
                key={s.id}
                type="button"
                role="tab"
                id={`settings-tab-${s.id}`}
                aria-selected={s.id === current.id}
                aria-controls="settings-page"
                tabIndex={s.id === current.id ? 0 : -1}
                className={`settings-tab${s.id === current.id ? " active" : ""}${s.preRelease ? " pre-release" : ""}`}
                ref={(el) => {
                  if (el) tabs.current.set(s.id, el);
                  else tabs.current.delete(s.id);
                }}
                onClick={() => choose(s.id)}
              >
                <Icon className="settings-tab-icon" size={16} stroke={1.75} aria-hidden="true" />
                <span className="settings-tab-label">{i18n._(s.label)}</span>
              </button>
            );
          })}
          </div>
        </nav>
        <div
          className="settings-page"
          id="settings-page"
          role="tabpanel"
          aria-labelledby={`settings-tab-${current.id}`}
          data-section={current.id}
        >
          <div className="settings-page-head">
            <h3 className="settings-page-title">{i18n._(current.label)}</h3>
            <button
              type="button"
              className="settings-close"
              onClick={onClose}
              aria-label={t({ message: "Close", comment: "Settings button: closes the settings window" })}
              title={t({ message: "Close", comment: "Settings button: closes the settings window" })}
            >
              <IconX size={16} stroke={1.75} aria-hidden="true" />
            </button>
          </div>
          <div className="settings-section">
            <Page config={config} update={update} />
          </div>
          <p className="settings-note">
            {t({
              message: "Changes apply as you make them.",
              comment: "Settings: a note at the bottom of each section; there is no Save button",
            })}
          </p>
        </div>
      </div>
    </div>
  );
}
