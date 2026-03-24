import { invoke } from "@tauri-apps/api/core";
import type {
  SearchResult,
  CompletionResult,
  MaximaFunction,
  CategoryGroup,
  PackageSearchResult,
  PackageCompletionResult,
  PackageInfo,
  PackageFunctionSearchResult,
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

// ── Package functions ────────────────────────────────────────────

export async function searchPackages(
  query: string
): Promise<PackageSearchResult[]> {
  return invoke<PackageSearchResult[]>("search_packages", { query });
}

export async function completePackages(
  prefix: string
): Promise<PackageCompletionResult[]> {
  return invoke<PackageCompletionResult[]>("complete_packages", { prefix });
}

export async function getPackage(
  name: string
): Promise<PackageInfo | null> {
  return invoke<PackageInfo | null>("get_package", { name });
}

export async function packageForFunction(
  name: string
): Promise<string | null> {
  return invoke<string | null>("package_for_function", { name });
}

export async function searchPackageFunctions(
  query: string
): Promise<PackageFunctionSearchResult[]> {
  return invoke<PackageFunctionSearchResult[]>("search_package_functions", { query });
}
