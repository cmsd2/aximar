// Which text colours sit on which backgrounds.  Each pair must reach 4.5:1
// in every theme (src/styles/tokens.test.ts), and the Foundations colours
// story shows the ratios.  Add a pair when a rule puts a text colour on a
// new background.

export interface ContrastPair {
  text: string;
  background: string;
  /** Why the pair needn't reach 4.5:1, for placeholders and decoration */
  exempt?: string;
}

const SURFACES = ["bg-primary", "bg-secondary", "bg-cell", "bg-input"];

const on = (text: string, backgrounds: string[]): ContrastPair[] =>
  backgrounds.map((background) => ({ text, background }));

export const CONTRAST_PAIRS: ContrastPair[] = [
  ...on("text-primary", [...SURFACES, "selected-bg"]),
  ...on("text-secondary", [...SURFACES, "selected-bg"]),
  // Links and quiet buttons on white surfaces only: on bg-secondary and
  // bg-cell the light accent falls just short
  ...on("accent", ["bg-primary", "bg-input"]),
  // A primary button's label
  { text: "bg-primary", background: "accent" },
  ...on("error-color", ["bg-primary", "bg-cell", "error-bg"]),
  ...on("success-color", ["bg-primary", "bg-cell"]),
  ...on("success-text", ["success-bg", "bg-primary"]),
  ...on("warning-text", ["warn-bg", "bg-primary"]),
  ...on("running-color", ["bg-primary", "bg-cell"]),
  // The editor's syntax colours
  ...on("syntax-builtin", ["bg-cell", "bg-input"]),
  ...on("syntax-string", ["bg-cell", "bg-input"]),
  ...on("syntax-number", ["bg-cell", "bg-input"]),
  ...on("text-placeholder", ["bg-primary", "bg-input"]).map((p) => ({
    ...p,
    exempt: "placeholder text and disabled items, which WCAG exempts",
  })),
];
