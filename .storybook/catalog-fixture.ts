// A few catalog entries for the stories: the docs panel, completion,
// argument help and package pages read these through the Tauri mock
import type { CompletionResult, MaximaFunction, PackageInfo } from "../src/types/catalog";

export const functions: Record<string, MaximaFunction> = {
  integrate: {
    name: "integrate",
    signatures: ["integrate(expr, x)", "integrate(expr, x, a, b)"],
    description: "Computes the indefinite or definite integral of expr with respect to x.",
    category: "Calculus",
    examples: [
      { input: "integrate(sin(x), x);", description: "An antiderivative" },
      { input: "integrate(exp(-x^2), x, minf, inf);", description: "A definite integral" },
    ],
    see_also: ["diff", "ldefint", "quad_qags"],
  },
  diff: {
    name: "diff",
    signatures: ["diff(expr, x)", "diff(expr, x, n)"],
    description: "Returns the derivative of expr with respect to x, or the nth derivative.",
    category: "Calculus",
    examples: [{ input: "diff(x^3, x);", description: null }],
    see_also: ["integrate", "depends"],
  },
  integerp: {
    name: "integerp",
    signatures: ["integerp(expr)"],
    description: "Returns true if expr is an integer literal, otherwise false.",
    category: "Programming",
    examples: [],
    see_also: [],
  },
};

export const docs: Record<string, string> = {
  integrate: [
    "Computes the integral of *expr* with respect to *x*. With limits *a* and *b* it returns the definite integral",
    "",
    "$$\\int_a^b f(x)\\,dx$$",
    "",
    "When Maxima finds no closed form it returns the integral unevaluated. Use `quad_qags` for a numerical value.",
    "",
    "```maxima",
    "(%i1) integrate(1/(1+x^2), x);",
    "(%o1)                              atan(x)",
    "```",
  ].join("\n"),
  diff: "Returns the derivative of *expr* with respect to *x*. `diff(expr, x, n)` returns the *n*th derivative.",
};

export const packages: Record<string, PackageInfo> = {
  distrib: {
    name: "distrib",
    description: "Probability distributions: densities, distribution functions, quantiles and random variates.",
    functions: ["pdf_normal", "cdf_normal", "quantile_normal", "random_normal"],
    signatures: {
      pdf_normal: "pdf_normal(x, m, s)",
      cdf_normal: "cdf_normal(x, m, s)",
      quantile_normal: "quantile_normal(q, m, s)",
      random_normal: "random_normal(m, s)",
    },
  },
};

export function completions(prefix: string): CompletionResult[] {
  const all: CompletionResult[] = [
    { name: "integrate", signature: "integrate(expr, x)", description: functions.integrate.description, insert_text: "integrate(" },
    { name: "integerp", signature: "integerp(expr)", description: functions.integerp.description, insert_text: "integerp(" },
    { name: "intersection", signature: "intersection(a_1, ..., a_n)", description: "Returns the set of elements common to all of its arguments.", insert_text: "intersection(" },
    { name: "interpolate", signature: "interpolate(f, x, a, b)", description: "Finds a zero of f between a and b.", insert_text: "interpolate(", package: "interpol" },
  ];
  return all.filter((c) => c.name.startsWith(prefix));
}
