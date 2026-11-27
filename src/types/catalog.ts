export type FunctionCategory =
  | "Calculus"
  | "Algebra"
  | "LinearAlgebra"
  | "Simplification"
  | "Solving"
  | "Plotting"
  | "Trigonometry"
  | "NumberTheory"
  | "Polynomials"
  | "Series"
  | "Combinatorics"
  | "Programming"
  | "IO"
  | "Other";

export interface FunctionExample {
  input: string;
  description: string | null;
}

export interface MaximaFunction {
  name: string;
  signatures: string[];
  description: string;
  category: FunctionCategory;
  examples: FunctionExample[];
  see_also: string[];
  search_keywords?: string;
  /** The languages of the text, from a lookup in the help language */
  languages?: DocLanguages;
}

/** The languages of a function's documentation */
export interface DocLanguages {
  /** The summary's language, such as es */
  summary: string;
  /** The full documentation's and examples' language */
  body: string;
  /** The help language, when the function has nothing in it */
  fallback_from?: string;
  /** A translation whose signatures differ from the English */
  stale: boolean;
  english_signatures?: string[];
}

export interface SearchResult {
  function: MaximaFunction;
  score: number;
}

export interface CompletionResult {
  name: string;
  signature: string;
  description: string;
  insert_text: string;
  /** If this function requires a package, the package name (e.g. "distrib"). */
  package?: string;
}

export interface CategoryGroup {
  category: FunctionCategory;
  label: string;
  functions: MaximaFunction[];
}

export interface PackageInfo {
  name: string;
  description: string;
  functions: string[];
  signatures?: Record<string, string>;
  /** Built-in packages are auto-loaded and don't need `load("...")$`. */
  builtin?: boolean;
}

export interface PackageSearchResult {
  package: PackageInfo;
  score: number;
}

export interface PackageCompletionResult {
  name: string;
  description: string;
}

export interface PackageFunctionSearchResult {
  function_name: string;
  package_name: string;
  package_description: string;
  score: number;
  signature: string;
}
