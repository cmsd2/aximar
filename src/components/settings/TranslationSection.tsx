// Pre-release builds only: tools for translators.  See docs/i18n.md.
import { useState } from "react";
import { useLingui } from "@lingui/react/macro";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import { currentPreviewPath, startPreview, stopPreview } from "../../lib/translation-preview";
import { setShowSources, showingSources } from "../../lib/show-sources";
import { SettingRow, type SectionProps } from "./SettingRow";

export function TranslationSection({ config }: SectionProps) {
  return (
    <>
      <TranslationPreviewRow language={config.language} />
      <ShowSourcesRow />
    </>
  );
}

/** Load a translation file from disk and see it in the window and the menus, again on each save */
function TranslationPreviewRow({ language }: { language: string }) {
  const { t } = useLingui();
  const [path, setPath] = useState<string | null>(currentPreviewPath());
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    const chosen = await openDialog({
      multiple: false,
      filters: [
        {
          name: t({ message: "Translation file", comment: "File type in the open dialog for a translation preview" }),
          extensions: ["po"],
        },
      ],
    });
    if (typeof chosen !== "string") return;
    try {
      await startPreview(chosen);
      setPath(currentPreviewPath());
      setError(null);
    } catch (e) {
      setError(String(e));
    }
  };

  const stop = async () => {
    await stopPreview(language);
    setPath(null);
  };

  return (
    <SettingRow
      label={t({
        message: "Translation preview",
        comment: "Settings, pre-release builds: shows a translation file from disk in the window as the translator saves it",
      })}
      hint={t({
        message: "Shows a .po file in the window and the menus, again each time you save it.",
        comment: "Settings, Translation section: hint under Translation preview",
      })}
    >
      {({ id, describedBy }) => (
        <div className="settings-claude-setup">
          {path ? (
            <>
              <span className="settings-claude-status" title={path}>
                {path.split(/[\\/]/).pop()}
              </span>
              <button id={id} aria-describedby={describedBy} type="button" className="settings-token-btn" onClick={() => void stop()}>
                {t({ message: "Stop preview", comment: "Settings: stops showing the translation file, going back to the language setting" })}
              </button>
            </>
          ) : (
            <button id={id} aria-describedby={describedBy} type="button" className="settings-token-btn" onClick={() => void load()}>
              {t({ message: "Load translation file...", comment: "Settings, pre-release builds: chooses a .po file to show in the window" })}
            </button>
          )}
          {error && <span className="settings-claude-status settings-claude-warn">{error}</span>}
        </div>
      )}
    </SettingRow>
  );
}

/** Hold Alt and point at text to see where it comes from */
function ShowSourcesRow() {
  const { t, i18n } = useLingui();
  return (
    <SettingRow
      label={t({
        message: "Show sources",
        comment: "Settings, pre-release builds: when on, holding Alt and pointing at text shows its English, note and source file",
      })}
      hint={t({
        message: "Hold Alt and point at text to see its English, note and source file",
        comment: "Tooltip of the Show sources checkbox in Settings",
      })}
    >
      {({ id, describedBy }) => (
        <input
          id={id}
          aria-describedby={describedBy}
          type="checkbox"
          className="settings-checkbox"
          checked={showingSources(i18n)}
          onChange={(e) => setShowSources(i18n, e.target.checked)}
        />
      )}
    </SettingRow>
  );
}
