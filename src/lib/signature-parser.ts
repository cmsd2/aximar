export interface ParsedSignature {
  name: string;
  params: string[];
  raw: string;
}

/**
 * Parse a signature string like "integrate(expr, var, lo, hi)" into structured data.
 * Handles bracket nesting so "[expr_1, ..., expr_m]" stays as one param.
 * Operators with no parens (like "!!") return params: [].
 */
export function parseSignature(sig: string): ParsedSignature {
  const raw = sig.trim();
  const openParen = raw.indexOf("(");
  const closeParen = raw.lastIndexOf(")");

  if (openParen === -1 || closeParen === -1 || closeParen <= openParen) {
    // No parens — operator or bare name
    return { name: raw, params: [], raw };
  }

  const name = raw.substring(0, openParen).trim();
  const inner = raw.substring(openParen + 1, closeParen);

  if (inner.trim() === "") {
    return { name, params: [], raw };
  }

  // Split on commas at depth 0 (tracking parens and brackets)
  const params: string[] = [];
  let current = "";
  let depth = 0;

  for (const ch of inner) {
    if (ch === "(" || ch === "[") {
      depth++;
      current += ch;
    } else if (ch === ")" || ch === "]") {
      depth--;
      current += ch;
    } else if (ch === "," && depth === 0) {
      params.push(current.trim());
      current = "";
    } else {
      current += ch;
    }
  }
  if (current.trim()) {
    params.push(current.trim());
  }

  return { name, params, raw };
}
