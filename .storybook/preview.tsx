import type { Decorator, Preview } from "@storybook/react-vite";
import { useEffect, useState } from "react";
import { I18nProvider } from "@lingui/react";
import { messages as pseudo } from "../locales/pseudo.po";
import { activateLanguage, i18n } from "../src/lib/i18n";
import { installSourceTags, setShowSources } from "../src/lib/show-sources";
import { installTauriMock } from "./tauri-mock";
import { applyStoryTheme } from "../src/stories/theme";
import { useNotebookStore, type CellStyle } from "../src/store/notebookStore";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/700.css";
import "../src/styles/global.css";

installTauriMock();
// Strings can carry their IDs, so the screenshot run can record which
// story shows which strings
installSourceTags(i18n);

/** Show the story in the toolbar's language and theme, tagging strings when asked */
const withLanguage: Decorator = (Story, context) => {
  applyStoryTheme(context.globals.theme);
  // Card style unless the story asks for bracket style
  // A story that needs a style asks for it; otherwise the toolbar's choice
  const cellStyle: CellStyle = context.parameters.cellStyle ?? (context.globals.cellStyle === "bracket" ? "bracket" : "card");
  document.documentElement.dataset.cellStyle = cellStyle;
  useNotebookStore.getState().setCellStyle(cellStyle);
  // Suggestion chips on unless the story turns them off
  useNotebookStore.getState().setShowSuggestions(context.parameters.showSuggestions ?? true);
  const language = context.globals.locale ?? "en";
  const tag = context.globals.tagStrings === "on";
  const [ready, setReady] = useState(false);
  useEffect(() => {
    let cancelled = false;
    (async () => {
      if (language === "pseudo") {
        i18n.load("pseudo", pseudo);
        i18n.activate("pseudo");
      } else {
        await activateLanguage(language);
      }
      setShowSources(i18n, tag);
      if (!cancelled) setReady(true);
    })();
    return () => {
      cancelled = true;
    };
  }, [language, tag]);
  if (!ready) return <></>;
  return (
    <I18nProvider i18n={i18n}>
      <div data-story-ready="true" style={{ padding: 16 }}>
        <Story />
      </div>
    </I18nProvider>
  );
};

const preview: Preview = {
  decorators: [withLanguage],
  globalTypes: {
    locale: {
      description: "Language",
      toolbar: {
        title: "Language",
        icon: "globe",
        items: [
          { value: "en", title: "English" },
          { value: "es", title: "Español" },
          { value: "pseudo", title: "Pseudo-locale" },
        ],
        dynamicTitle: true,
      },
    },
    theme: {
      description: "Theme",
      toolbar: {
        title: "Theme",
        icon: "mirror",
        items: [
          { value: "light", title: "Light", icon: "sun" },
          { value: "paper", title: "Paper", icon: "document" },
          { value: "dark", title: "Dark", icon: "moon" },
        ],
        dynamicTitle: true,
      },
    },
    cellStyle: {
      description: "Cell style",
      toolbar: {
        title: "Cells",
        icon: "component",
        items: [
          { value: "card", title: "Card" },
          { value: "bracket", title: "Bracket" },
        ],
        dynamicTitle: true,
      },
    },
    tagStrings: {
      description: "Tag strings with their IDs, for the screenshot run",
      toolbar: { title: "Tags", items: ["off", "on"] },
    },
  },
  initialGlobals: { locale: "en", theme: "light", cellStyle: "card", tagStrings: "off" },
  parameters: { layout: "fullscreen" },
};

export default preview;
