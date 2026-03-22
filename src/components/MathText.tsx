import { useMemo } from "react";
import katex from "katex";

interface MathTextProps {
  text: string;
  className?: string;
}

/**
 * Renders text containing inline ($...$) and display ($$...$$) LaTeX math
 * using KaTeX, with plain text segments rendered as-is.
 */
export function MathText({ text, className }: MathTextProps) {
  const parts = useMemo(() => splitMath(text), [text]);

  if (parts.length === 1 && parts[0].type === "text") {
    return <span className={className}>{text}</span>;
  }

  return (
    <span className={className}>
      {parts.map((part, i) => {
        if (part.type === "text") {
          return <span key={i}>{part.content}</span>;
        }
        const displayMode = part.type === "display";
        try {
          const html = katex.renderToString(part.content, {
            displayMode,
            throwOnError: false,
            trust: true,
          });
          return (
            <span
              key={i}
              dangerouslySetInnerHTML={{ __html: html }}
            />
          );
        } catch {
          return <code key={i}>{part.content}</code>;
        }
      })}
    </span>
  );
}

type Part =
  | { type: "text"; content: string }
  | { type: "inline"; content: string }
  | { type: "display"; content: string };

/** Split text into plain text, inline math ($...$), and display math ($$...$$). */
function splitMath(text: string): Part[] {
  const parts: Part[] = [];
  let i = 0;

  while (i < text.length) {
    // Check for display math ($$...$$)
    if (text[i] === "$" && text[i + 1] === "$") {
      const end = text.indexOf("$$", i + 2);
      if (end !== -1) {
        if (i > 0) {
          const prev = parts.length > 0 ? parts[parts.length - 1] : null;
          if (!prev || prev.type !== "text") {
            // Already flushed
          }
        }
        parts.push({ type: "display", content: text.slice(i + 2, end) });
        i = end + 2;
        continue;
      }
    }

    // Check for inline math ($...$)
    if (text[i] === "$") {
      const end = text.indexOf("$", i + 1);
      if (end !== -1) {
        parts.push({ type: "inline", content: text.slice(i + 1, end) });
        i = end + 1;
        continue;
      }
    }

    // Plain text — accumulate until next $
    const next = text.indexOf("$", i);
    const chunk = next === -1 ? text.slice(i) : text.slice(i, next);
    if (chunk) {
      const last = parts[parts.length - 1];
      if (last && last.type === "text") {
        last.content += chunk;
      } else {
        parts.push({ type: "text", content: chunk });
      }
    }
    i = next === -1 ? text.length : next;
  }

  return parts;
}
