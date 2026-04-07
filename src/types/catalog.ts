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
