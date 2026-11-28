import React from "react";
// The monospace font ships with the app; its licence is public/licenses/JetBrainsMono-OFL.txt
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/700.css";
import ReactDOM from "react-dom/client";
import { I18nProvider } from "@lingui/react";
import App from "./App";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { i18n, activateDevelopmentLocale } from "./lib/i18n";
import { installSourceTags } from "./lib/show-sources";
import { TranslatorTools } from "./components/TranslatorTools";
import { currentPlatform } from "./commands/shortcut";

// Strings can carry their IDs, for show-sources; only while it's on
installSourceTags(i18n);

// Styles that follow the platform, such as Windows 11's type (tokens.css)
document.documentElement.dataset.platform = currentPlatform();

// Async errors (rejected promises from Plotly, IPC, etc.) are not caught by
// React error boundaries. Log them instead of letting them surface as an
// uncaught crash in the webview.
window.addEventListener("unhandledrejection", (event) => {
  console.error("[unhandledrejection]", event.reason);
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <I18nProvider i18n={i18n}>
      <ErrorBoundary>
        <App />
      </ErrorBoundary>
      <TranslatorTools />
    </I18nProvider>
  </React.StrictMode>,
);

// The provider re-renders the window when the language changes
void activateDevelopmentLocale();
