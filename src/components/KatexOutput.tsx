import { useMemo } from "react";
import katex from "katex";
import { preprocessLatex } from "../lib/katex-helpers";

interface KatexOutputProps {
  latex: string;
}

export function KatexOutput({ latex }: KatexOutputProps) {
  const html = useMemo(() => {
    try {
      const processed = preprocessLatex(latex);
      return katex.renderToString(processed, {
        displayMode: true,
        throwOnError: false,
        trust: true,
      });
    } catch {
      return `<span>LaTeX render error: ${latex}</span>`;
    }
  }, [latex]);

  return (
    <div
      className="katex-output"
      dangerouslySetInnerHTML={{ __html: html }}
    />
  );
}
