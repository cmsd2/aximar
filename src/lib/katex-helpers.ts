/**
 * Preprocess Maxima's tex() output for KaTeX compatibility.
 */
export function preprocessLatex(latex: string): string {
  let result = latex;

  // Replace {\it content} with \mathit{content}
  // Maxima wraps \it in braces: {\it \%k} → \mathit{\%k}
  result = result.replace(/\{\\it\s+([^}]*)\}/g, "\\mathit{$1}");

  // Strip Maxima's \ifx\endpmatrix\undefined...\else...\fi conditionals.
  // Maxima emits these to support both plain TeX and LaTeX — we keep the
  // LaTeX branch (\begin{pmatrix} / \end{pmatrix}).
  result = result.replace(
    /\\ifx\\endpmatrix\\undefined\\pmatrix\{\\else\\begin\{pmatrix\}\\fi/g,
    "\\begin{pmatrix}"
  );
  result = result.replace(
    /\\ifx\\endpmatrix\\undefined\}\\else\\end\{pmatrix\}\\fi/g,
    "\\end{pmatrix}"
  );

  // Replace \cr row separators with \\ (strip trailing \cr before \end)
  result = result.replace(/\\cr\s*\\end\{pmatrix\}/g, "\\end{pmatrix}");
  result = result.replace(/\\cr/g, "\\\\");

  // Replace \mbox with \text — Maxima uses \mbox for keywords like
  // "if", "then", "else"; \text is better supported in KaTeX.
  // (Pure string results like \mbox{...} are filtered out by the parser.)
  result = result.replace(/\\mbox\{/g, "\\text{");

  // Handle any remaining plain \pmatrix{...} (older Maxima versions)
  result = replacePmatrix(result);

  return result;
}

/**
 * Find \pmatrix{...} with balanced brace matching and convert to
 * \begin{pmatrix}...\end{pmatrix}, replacing \cr row separators with \\.
 */
function replacePmatrix(latex: string): string {
  const prefix = "\\pmatrix{";
  let result = "";
  let i = 0;

  while (i < latex.length) {
    const idx = latex.indexOf(prefix, i);
    if (idx === -1) {
      result += latex.substring(i);
      break;
    }

    result += latex.substring(i, idx);
    const contentStart = idx + prefix.length;

    // Walk forward counting braces to find the matching close
    let depth = 1;
    let j = contentStart;
    while (j < latex.length && depth > 0) {
      if (latex[j] === "{") depth++;
      else if (latex[j] === "}") depth--;
      j++;
    }

    if (depth === 0) {
      let content = latex.substring(contentStart, j - 1);
      // Replace \cr row separators with \\ and strip trailing one
      content = content.replace(/\\cr\s*$/, "");
      content = content.replace(/\\cr/g, "\\\\");
      result += "\\begin{pmatrix}" + content + "\\end{pmatrix}";
      i = j;
    } else {
      // Unbalanced — keep original text and move past the prefix
      result += prefix;
      i = contentStart;
    }
  }

  return result;
}
