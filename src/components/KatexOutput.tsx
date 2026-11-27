import { useMemo } from "react";
import katex from "katex";
import { useLingui } from "@lingui/react/macro";
import { preprocessLatex } from "../lib/katex-helpers";

interface KatexOutputProps {
  latex: string;
}

export function KatexOutput({ latex }: KatexOutputProps) {
  const { t } = useLingui();
  const html = useMemo(() => {
    try {
      const processed = preprocessLatex(latex);
      console.debug("[KaTeX] raw:", latex, "→ processed:", processed);
      return katex.renderToString(processed, {
        displayMode: true,
        throwOnError: false,
        trust: false,
      });
    } catch (e) {
      console.warn("[KaTeX] render failed:", e, "raw:", latex);
      return null;
    }
  }, [latex]);

  if (html === null) {
    return (
      <div className="katex-output">
        <span>
          {t({
            message: `LaTeX render error: ${latex}`,
            comment: "Shown in place of a result whose LaTeX could not be drawn. latex is the LaTeX source, which stays as it is",
          })}
        </span>
      </div>
    );
  }

  return (
    <div
      className="katex-output"
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}
