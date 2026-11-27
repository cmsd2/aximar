import { useMemo } from "react";
import katex from "katex";
import { preprocessLatex } from "../lib/katex-helpers";

interface RichTextOutputProps {
  text: string;
}

/** Split text_output on $$...$$ blocks, rendering LaTeX segments with KaTeX. */
export function RichTextOutput({ text }: RichTextOutputProps) {
  const segments = useMemo(() => {
    // Split preserving $$...$$ delimiters as separate segments: the odd
    // parts are the blocks
    const raw = text.split(/(\$\$[\s\S]*?\$\$)/);
    const parts = raw.map((part, i) => {
      if (i % 2 === 1) return part;
      // Maxima prints tex() output on lines of its own.  The block breaks
      // the line itself, so the newline after one would show as a blank
      // line under it.
      let out = part;
      if (i > 0) out = out.replace(/^\n/, "");
      if (i < raw.length - 1) out = out.replace(/\n$/, "");
      return out;
    });
    return parts
      .map((part, i) => ({ part, i }))
      .filter(({ part }) => part !== "")
      .map(({ part, i }) => {
        if (i % 2 === 1) {
          const inner = part.slice(2, -2);
          try {
            const processed = preprocessLatex(inner);
            const html = katex.renderToString(processed, {
              displayMode: true,
              throwOnError: false,
              trust: false,
            });
            return { type: "latex" as const, html, key: i };
          } catch {
            return { type: "text" as const, text: part, key: i };
          }
        }
        return { type: "text" as const, text: part, key: i };
      });
  }, [text]);

  // If no LaTeX segments, render as plain pre (unchanged look)
  const hasLatex = segments.some((s) => s.type === "latex");
  if (!hasLatex) {
    return <pre className="text-output">{text}</pre>;
  }

  return (
    <div className="text-output rich-text-output">
      {segments.map((seg) =>
        seg.type === "latex" ? (
          <div
            key={seg.key}
            className="katex-output inline-katex"
            dangerouslySetInnerHTML={{ __html: seg.html }}
          />
        ) : (
          <pre key={seg.key} className="text-segment">
            {seg.text}
          </pre>
        ),
      )}
    </div>
  );
}
