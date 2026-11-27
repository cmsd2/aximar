// The licences Aximar accepts for code and fonts it ships, in order of
// preference.  A package offering a choice is used under the first of
// these it offers.  A licence outside the list stops the build: see
// docs/licences.md.  about.toml holds the same list for the Rust crates.

export const ACCEPTED_LICENCES = [
  "MIT",
  "Apache-2.0",
  "Apache-2.0 WITH LLVM-exception",
  "BSD-3-Clause",
  "BSD-2-Clause",
  "ISC",
  "Zlib",
  "0BSD",
  "MIT-0",
  "Unicode-3.0",
  "CC0-1.0",
  "BSL-1.0",
  "Unlicense",
  "OFL-1.1",
  "MPL-2.0",
] as const;

type Token = "(" | ")" | "AND" | "OR" | "WITH" | { id: string };

/** An SPDX expression as tokens; "MIT/Apache-2.0", an older npm form, reads as an OR */
function tokens(expression: string): Token[] {
  const out: Token[] = [];
  for (const word of expression.replace(/\s*\/\s*/g, " OR ").match(/\(|\)|[^\s()]+/g) ?? []) {
    const upper = word.toUpperCase();
    if (word === "(" || word === ")") out.push(word);
    else if (upper === "AND" || upper === "OR" || upper === "WITH") out.push(upper);
    else out.push({ id: word });
  }
  return out;
}

/** The accepted licences a package needs, under the choice it prefers, or null */
type Choice = string[] | null;

const rank = (choice: string[]) =>
  Math.max(...choice.map((id) => ACCEPTED_LICENCES.indexOf(id as (typeof ACCEPTED_LICENCES)[number])));

function parse(input: Token[]): Choice {
  let at = 0;
  const peek = () => input[at];

  function atom(): Choice {
    const t = input[at++];
    if (t === "(") {
      const inner = or();
      if (input[at++] !== ")") throw new Error("unbalanced parentheses");
      return inner;
    }
    if (typeof t !== "object") throw new Error(`unexpected ${String(t)}`);
    let id = t.id;
    if (peek() === "WITH") {
      at++;
      const exception = input[at++];
      if (typeof exception !== "object") throw new Error("WITH needs an exception");
      id = `${id} WITH ${exception.id}`;
    }
    const accepted = ACCEPTED_LICENCES.find((a) => a.toLowerCase() === id.toLowerCase());
    return accepted ? [accepted] : null;
  }

  function and(): Choice {
    let all: Choice = atom();
    while (peek() === "AND") {
      at++;
      const next = atom();
      all = all && next ? Array.from(new Set(all.concat(next))) : null;
    }
    return all;
  }

  function or(): Choice {
    let best: Choice = and();
    while (peek() === "OR") {
      at++;
      const next = and();
      if (next && (!best || rank(next) < rank(best))) best = next;
    }
    return best;
  }

  const result = or();
  if (at !== input.length) throw new Error("unexpected text after the expression");
  return result;
}

/**
 * The licence, or licences joined by AND, a package is used under: the
 * accepted choice its SPDX expression prefers.  Null when the expression
 * offers no accepted choice, is missing, or can't be read.
 */
export function chosenLicence(expression: string | null | undefined): string | null {
  if (!expression?.trim()) return null;
  try {
    return parse(tokens(expression))?.join(" AND ") ?? null;
  } catch {
    return null;
  }
}

export function isAccepted(expression: string | null | undefined): boolean {
  return chosenLicence(expression) !== null;
}
