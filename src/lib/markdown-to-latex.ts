/**
 * Markdown → LaTeX converter using remark (mdast) for robust parsing.
 *
 * Uses the same remark parser + plugins the app already uses for rendering
 * (remark-gfm for tables, remark-math for $...$ and $$...$$), so the
 * LaTeX export matches what users see in the notebook.
 */

import { unified } from "unified";
import remarkParse from "remark-parse";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import type { Root, Content, Table, TableRow } from "mdast";

const parser = unified().use(remarkParse).use(remarkGfm).use(remarkMath);

export function markdownToLatex(md: string): string {
  const tree = parser.parse(md) as Root;
  return visitChildren(tree.children);
}

// ── AST walkers ──────────────────────────────────────────────────

function visitChildren(nodes: Content[]): string {
  return nodes.map(visitNode).join("\n\n");
}

function visitNode(node: Content): string {
  switch (node.type) {
    case "heading": {
      const cmds = ["section", "subsection", "subsubsection", "paragraph"];
      const cmd = cmds[Math.min(node.depth - 1, cmds.length - 1)];
      return `\\${cmd}{${visitInline(node.children)}}`;
    }

    case "paragraph":
      return visitInline(node.children);

    case "blockquote":
      return (
        "\\begin{quote}\n" +
        visitChildren(node.children as Content[]) +
        "\n\\end{quote}"
      );

    case "code":
      return (
        "\\begin{verbatim}\n" + node.value + "\n\\end{verbatim}"
      );

    case "list": {
      const env = node.ordered ? "enumerate" : "itemize";
      const items = (node.children as Content[])
        .map((child) => {
          if (child.type === "listItem") {
            const inner = visitChildren(child.children as Content[]);
            return `  \\item ${inner}`;
          }
          return "";
        })
        .join("\n");
      return `\\begin{${env}}\n${items}\n\\end{${env}}`;
    }

    case "table":
      return convertTable(node as Table);

    case "thematicBreak":
      return "\\bigskip\\hrule\\bigskip";

    case "math":
      // Display math ($$...$$)
      return "\\[\n" + node.value + "\n\\]";

    case "html":
      // Drop raw HTML
      return "";

    default:
      // Inline-level nodes that can appear at block level (rare)
      if ("children" in node) {
        return visitInline((node as { children: Content[] }).children);
      }
      if ("value" in node) {
        return escapeLatex((node as { value: string }).value);
      }
      return "";
  }
}

/** Render an array of inline (phrasing) nodes to a LaTeX string. */
function visitInline(nodes: Content[]): string {
  return nodes.map(visitInlineNode).join("");
}

function visitInlineNode(node: Content): string {
  switch (node.type) {
    case "text":
      return escapeLatex(node.value);

    case "strong":
      return `\\textbf{${visitInline(node.children as Content[])}}`;

    case "emphasis":
      return `\\textit{${visitInline(node.children as Content[])}}`;

    case "inlineCode":
      return verb(node.value);

    case "inlineMath":
      return `$${node.value}$`;

    case "link":
      return `\\href{${node.url}}{${visitInline(node.children as Content[])}}`;

    case "image":
      return `\\includegraphics{${node.url}}`;

    case "break":
      return " \\\\\n";

    case "delete":
      // Strikethrough — no standard LaTeX equivalent, use sout if available
      return `\\sout{${visitInline(node.children as Content[])}}`;

    default:
      if ("children" in node) {
        return visitInline((node as { children: Content[] }).children);
      }
      if ("value" in node) {
        return escapeLatex((node as { value: string }).value);
      }
      return "";
  }
}

// ── Tables ───────────────────────────────────────────────────────

function convertTable(node: Table): string {
  const rows = node.children as TableRow[];
  if (rows.length === 0) return "";

  const colCount = Math.max(...rows.map((r) => r.children.length));

  // Build column alignment from node.align
  const aligns = (node.align || []).map((a) =>
    a === "center" ? "c" : a === "right" ? "r" : "l",
  );
  while (aligns.length < colCount) aligns.push("l");
  const colSpec = aligns.join("");

  const lines: string[] = [];
  lines.push(`\\begin{tabular}{${colSpec}}`);
  lines.push("\\hline");

  for (let ri = 0; ri < rows.length; ri++) {
    const cells = rows[ri].children.map((cell) =>
      visitInline(cell.children as Content[]),
    );
    while (cells.length < colCount) cells.push("");
    lines.push(cells.join(" & ") + " \\\\");
    if (ri === 0) lines.push("\\hline"); // header separator
  }

  lines.push("\\hline");
  lines.push("\\end{tabular}");
  return lines.join("\n");
}

// ── Escaping ─────────────────────────────────────────────────────

/** Escape LaTeX special characters in plain text. */
function escapeLatex(text: string): string {
  return text.replace(/[\\{}$&#%_~^]/g, (ch) => {
    switch (ch) {
      case "\\":
        return "\\textbackslash{}";
      case "~":
        return "\\textasciitilde{}";
      case "^":
        return "\\textasciicircum{}";
      default:
        return "\\" + ch;
    }
  });
}

/**
 * Wrap inline code in \verb with a delimiter that doesn't appear in the
 * content. Falls back to \texttt with escaping if no safe delimiter exists.
 */
function verb(code: string): string {
  // \verb requires a single-character delimiter not present in the content
  const delimiters = "|!@+=/;:";
  for (const d of delimiters) {
    if (!code.includes(d)) {
      return `\\verb${d}${code}${d}`;
    }
  }
  // Fallback: escape manually for \texttt
  return "\\texttt{" + escapeLatex(code) + "}";
}
