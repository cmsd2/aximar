import { useMemo } from "react";
import katex from "katex";
import { preprocessLatex } from "../lib/katex-helpers";

interface RichTextOutputProps {
  text: string;
}

/** Split text_output on $$...$$ blocks, rendering LaTeX segments with KaTeX. */
export function RichTextOutput({ text }: RichTextOutputProps) {
  const segments = useMemo(() => {
    // Split preserving $$...$$ delimiters as separate segments
    const parts = text.split(/(\$\$[\s\S]*?\$\$)/);
    return parts
      .filter((p) => p !== "")
      .map((part, i) => {
        if (part.startsWith("$$") && part.endsWith("$$")) {
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
