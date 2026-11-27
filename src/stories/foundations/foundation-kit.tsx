// Shared parts of the Foundations stories: the parsed tokens and a few
// layout pieces drawn with the tokens themselves.
import type { CSSProperties, ReactNode } from "react";
import tokensCss from "../../styles/tokens.css?raw";
import { isColour, parseTokens } from "../../styles/tokens";

export const tokens = parseTokens(tokensCss);

/** The scale tokens of `:root` (not colours) whose names start with one of `prefixes`, in file order */
export function tokensNamed(...prefixes: string[]): [string, string][] {
  return [...tokens.light].filter(([name, value]) => !isColour(value) && prefixes.some((p) => name.startsWith(p)));
}

/** The colour tokens, in file order */
export function colourTokens(): [string, string][] {
  return [...tokens.light].filter(([, value]) => isColour(value));
}

export const page: CSSProperties = {
  fontFamily: "var(--font-ui)",
  color: "var(--text-primary)",
  maxWidth: 960,
};

export function Heading({ children, note }: { children: ReactNode; note?: ReactNode }) {
  return (
    <header style={{ marginBottom: "var(--space-4)" }}>
      <h1 style={{ fontSize: "var(--text-2xl)", fontWeight: 600, margin: 0 }}>{children}</h1>
      {note && (
        <p style={{ fontSize: "var(--text-md)", color: "var(--text-secondary)", margin: "var(--space-1) 0 0" }}>{note}</p>
      )}
    </header>
  );
}

export function Name({ children }: { children: ReactNode }) {
  return <code style={{ fontFamily: "var(--font-mono)", fontSize: "var(--text-sm)" }}>{children}</code>;
}

export const cell: CSSProperties = {
  padding: "var(--space-2) var(--space-3)",
  borderBottom: "0.5px solid var(--border-color)",
  fontSize: "var(--text-md)",
  textAlign: "left",
  verticalAlign: "middle",
};

export const headCell: CSSProperties = {
  ...cell,
  fontSize: "var(--text-sm)",
  fontWeight: 500,
  color: "var(--text-secondary)",
};
