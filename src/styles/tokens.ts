// Reading src/styles/tokens.css: its blocks and their variables, for the
// contrast test and the Foundations stories.

/** A token's name without the leading dashes, and its value as written */
export type TokenMap = Map<string, string>;

export interface Tokens {
  /** `:root`: the light colours and every scale */
  light: TokenMap;
  /** The dark block under `prefers-color-scheme: dark` */
  darkAuto: TokenMap;
  /** The dark block under `[data-theme="dark"]` */
  dark: TokenMap;
  /** The print palette, under `@media print` */
  print: TokenMap;
  /** Each token's comment heading in `:root`, such as "Spacing, on a 4px grid" */
  groups: Map<string, string>;
}

/** The text between the braces of the block that `opener` starts */
function blockAfter(css: string, opener: string): string {
  const at = css.indexOf(opener);
  if (at < 0) throw new Error(`tokens.css has no block ${opener}`);
  let i = css.indexOf("{", at) + 1;
  const start = i;
  for (let depth = 1; depth > 0; i++) {
    if (css[i] === "{") depth++;
    else if (css[i] === "}") depth--;
  }
  return css.slice(start, i - 1);
}

function variables(body: string): TokenMap {
  const map: TokenMap = new Map();
  for (const m of body.matchAll(/--([a-z0-9-]+):\s*([^;]+?)\s*(?:!important)?\s*;/g)) map.set(m[1], m[2]);
  return map;
}

export function parseTokens(css: string): Tokens {
  const rootBody = blockAfter(css, ":root {");
  const groups = new Map<string, string>();
  let heading = "";
  for (const line of rootBody.split("\n")) {
    const comment = line.match(/^\s*\/\*\s*(.+?)\s*\*\/\s*$/);
    if (comment) heading = comment[1];
    const name = line.match(/^\s*--([a-z0-9-]+):/);
    if (name) groups.set(name[1], heading);
  }
  const autoDark = blockAfter(css, "@media (prefers-color-scheme: dark)");
  return {
    light: variables(rootBody),
    darkAuto: variables(blockAfter(autoDark, ':root:not([data-theme="light"])')),
    dark: variables(blockAfter(css, '[data-theme="dark"]')),
    print: variables(blockAfter(css, "@media print")),
    groups,
  };
}

/** Whether a token's value is a colour written as six-digit hex */
export function isColour(value: string): boolean {
  return /^#[0-9a-fA-F]{6}$/.test(value);
}

function luminance(hex: string): number {
  const channel = (i: number) => {
    const c = parseInt(hex.slice(i, i + 2), 16) / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * channel(1) + 0.7152 * channel(3) + 0.0722 * channel(5);
}

/** WCAG's contrast ratio between two six-digit hex colours, from 1 to 21 */
export function contrast(a: string, b: string): number {
  const [hi, lo] = [luminance(a), luminance(b)].sort((x, y) => y - x);
  return (hi + 0.05) / (lo + 0.05);
}
