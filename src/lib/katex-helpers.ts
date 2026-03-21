/**
 * Preprocess Maxima's tex() output for KaTeX compatibility.
 */
export function preprocessLatex(latex: string): string {
  let result = latex;

  // Replace \it with \mathit
  result = result.replace(/\\it\s+/g, "\\mathit{");
  // Close \mathit braces at word boundary
  if (result.includes("\\mathit{")) {
    result = result.replace(/\\mathit\{([a-zA-Z_]+)/g, "\\mathit{$1}");
  }

  // Replace \pmatrix{...} with \begin{pmatrix}...\end{pmatrix}
  result = result.replace(
    /\\pmatrix\{([^}]*)\}/g,
    "\\begin{pmatrix}$1\\end{pmatrix}"
  );

  return result;
}
