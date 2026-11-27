import { useEffect, useState } from "react";
import { useLingui } from "@lingui/react/macro";
import { decodeIds, showingSources, type Source } from "../lib/show-sources";

interface Shown {
  sources: Source[];
  x: number;
  y: number;
}

/**
 * Pre-release builds, while showing sources: holding Alt and pointing at
 * text shows where each string in it comes from
 */
export function SourcesOverlay({ sources }: { sources: Map<string, Source> }) {
  const { i18n, t } = useLingui();
  const [shown, setShown] = useState<Shown | null>(null);
  const on = showingSources(i18n);

  useEffect(() => {
    if (!on) {
      setShown(null);
      return;
    }
    const onOver = (e: MouseEvent) => {
      if (!e.altKey) {
        setShown(null);
        return;
      }
      const found = sourcesAt(e.target as Element | null, sources);
      setShown(found.length > 0 ? { sources: found, x: e.clientX, y: e.clientY } : null);
    };
    document.addEventListener("mouseover", onOver);
    return () => document.removeEventListener("mouseover", onOver);
  }, [on, sources]);

  if (!shown) return null;
  return (
    <div className="sources-overlay" style={{ left: shown.x + 12, top: shown.y + 12 }}>
      {shown.sources.map((s) => (
        <div className="sources-entry" key={s.id}>
          <div className="sources-msgid">{s.msgid}</div>
          {s.context && (
            <div className="sources-detail">
              {t({ message: "Context:", comment: "Show-sources panel: before a string's msgctxt" })} {s.context}
            </div>
          )}
          {s.notes.map((n) => (
            <div className="sources-detail sources-note" key={n}>
              {n}
            </div>
          ))}
          {s.files.map((f) => (
            <div className="sources-detail sources-file" key={f}>
              {f}
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}

/**
 * The strings in the innermost element under the pointer that has any,
 * including those in its title and placeholder
 */
function sourcesAt(target: Element | null, sources: Map<string, Source>): Source[] {
  for (let el = target; el; el = el.parentElement) {
    const text = [el.textContent ?? "", el.getAttribute("title") ?? "", el.getAttribute("placeholder") ?? ""].join(" ");
    const ids = decodeIds(text);
    if (ids.length > 0) {
      return [...new Set(ids)].map((id) => sources.get(id)).filter((s): s is Source => !!s);
    }
  }
  return [];
}
