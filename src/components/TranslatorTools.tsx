import { useEffect, useState } from "react";
import { useLingui } from "@lingui/react/macro";
import { showingSources, type Source } from "../lib/show-sources";
import { loadSources } from "../lib/translation-preview";
import { SourcesOverlay } from "./SourcesOverlay";

/** The show-sources overlay, with the template loaded on first use */
export function TranslatorTools() {
  const { i18n } = useLingui();
  const [sources, setSources] = useState<Map<string, Source> | null>(null);
  const on = showingSources(i18n);

  useEffect(() => {
    if (on && !sources) void loadSources().then(setSources);
  }, [on, sources]);

  return sources ? <SourcesOverlay sources={sources} /> : null;
}
