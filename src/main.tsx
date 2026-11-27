import React from "react";
import ReactDOM from "react-dom/client";
import { I18nProvider } from "@lingui/react";
import App from "./App";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { i18n, activateDevelopmentLocale } from "./lib/i18n";
import { TRANSLATOR_TOOLS } from "./lib/translation-preview";
import { installSourceTags } from "./lib/show-sources";
import { TranslatorTools } from "./components/TranslatorTools";

// Pre-release builds: strings can carry their IDs, for show-sources
if (TRANSLATOR_TOOLS) installSourceTags(i18n);

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
      {TRANSLATOR_TOOLS && <TranslatorTools />}
    </I18nProvider>
  </React.StrictMode>,
);

// The provider re-renders the window when the language changes
void activateDevelopmentLocale();
