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
}

export interface CategoryGroup {
  category: FunctionCategory;
  label: string;
  functions: MaximaFunction[];
}
