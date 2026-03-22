import { invoke } from "@tauri-apps/api/core";
import type {
  SearchResult,
  CompletionResult,
  MaximaFunction,
  CategoryGroup,
} from "../types/catalog";

export async function searchFunctions(
  query: string
): Promise<SearchResult[]> {
  return invoke<SearchResult[]>("search_functions", { query });
}

export async function completeFunction(
  prefix: string
): Promise<CompletionResult[]> {
  return invoke<CompletionResult[]>("complete_function", { prefix });
}

export async function getFunction(
  name: string
): Promise<MaximaFunction | null> {
  return invoke<MaximaFunction | null>("get_function", { name });
}

export async function listCategories(): Promise<CategoryGroup[]> {
  return invoke<CategoryGroup[]>("list_categories");
}

export async function getFunctionDocs(
  name: string
): Promise<string | null> {
  return invoke<string | null>("get_function_docs", { name });
}
