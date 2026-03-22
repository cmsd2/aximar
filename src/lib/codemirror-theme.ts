import { EditorView } from "@codemirror/view";
import { HighlightStyle, syntaxHighlighting } from "@codemirror/language";
import { tags } from "@lezer/highlight";

export const maximaTheme = EditorView.theme({
  "&": {
    color: "var(--text-primary)",
    backgroundColor: "transparent",
    fontSize: "var(--font-size-mono)",
    fontFamily: '"JetBrains Mono", "Fira Code", "SF Mono", Menlo, Consolas, monospace',
  },
  "&.cm-focused": {
    outline: "none",
  },
  ".cm-scroller": {
    overflow: "visible",
    lineHeight: "1.5",
    fontFamily: "inherit",
  },
  ".cm-content": {
    padding: "0",
    caretColor: "var(--text-primary)",
  },
  ".cm-line": {
    padding: "0",
  },
  ".cm-cursor, .cm-dropCursor": {
    borderLeftColor: "var(--text-primary)",
    borderLeftWidth: "2px",
  },
  "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection": {
    backgroundColor: "var(--accent)",
    opacity: "0.3",
  },
  ".cm-selectionMatch": {
    backgroundColor: "rgba(var(--accent), 0.2)",
  },
  ".cm-placeholder": {
    color: "var(--text-placeholder)",
    fontStyle: "normal",
  },
  // Autocomplete popup styling
  ".cm-tooltip.cm-tooltip-autocomplete": {
    background: "var(--bg-primary)",
    border: "1px solid var(--border-color)",
    borderRadius: "8px",
    boxShadow: "0 8px 24px rgba(0, 0, 0, 0.15)",
    padding: "4px 0",
    overflow: "hidden",
  },
  ".cm-tooltip.cm-tooltip-autocomplete > ul": {
    maxHeight: "240px",
    minWidth: "260px",
    maxWidth: "400px",
    fontFamily: '"JetBrains Mono", "Fira Code", "SF Mono", Menlo, Consolas, monospace',
    fontSize: "13px",
  },
  ".cm-tooltip.cm-tooltip-autocomplete > ul > li": {
    padding: "6px 12px",
    display: "flex",
    gap: "8px",
    alignItems: "baseline",
  },
  ".cm-tooltip.cm-tooltip-autocomplete > ul > li[aria-selected]": {
    background: "var(--accent)",
    color: "var(--bg-primary)",
  },
  ".cm-completionLabel": {
    fontWeight: "600",
  },
  ".cm-completionDetail": {
    fontSize: "11px",
    color: "var(--text-secondary)",
    fontStyle: "normal",
    marginLeft: "auto",
    whiteSpace: "nowrap",
    overflow: "hidden",
    textOverflow: "ellipsis",
  },
  ".cm-tooltip.cm-tooltip-autocomplete > ul > li[aria-selected] .cm-completionDetail": {
    color: "inherit",
    opacity: "0.8",
  },
  // General tooltip styling (signature hints, hover)
  ".cm-tooltip": {
    background: "var(--bg-primary)",
    border: "1px solid var(--border-color)",
    borderRadius: "8px",
    boxShadow: "0 8px 24px rgba(0, 0, 0, 0.15)",
    zIndex: "1000",
  },
});

export const maximaHighlightStyle = syntaxHighlighting(
  HighlightStyle.define([
    { tag: tags.keyword, color: "var(--accent)" },
    { tag: tags.variableName, color: "var(--text-primary)" },
    { tag: tags.standard(tags.variableName), color: "#b8860b", fontWeight: "500" },
    { tag: tags.string, color: "#2d8a4e" },
    { tag: tags.number, color: "#9b59b6" },
    { tag: tags.operator, color: "var(--text-secondary)" },
    { tag: tags.blockComment, color: "var(--text-secondary)", fontStyle: "italic" },
  ])
);
