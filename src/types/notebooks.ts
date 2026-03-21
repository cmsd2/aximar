export interface NotebookCell {
  cell_type: "code" | "markdown" | "raw";
  source: string | string[];
  metadata: Record<string, unknown>;
  execution_count?: number | null;
  outputs?: unknown[];
}

export interface KernelSpec {
  name: string;
  display_name: string;
  language?: string;
}

export interface AximarMetadata {
  template_id?: string;
  title?: string;
  description?: string;
}

export interface NotebookMetadata {
  kernelspec: KernelSpec;
  aximar?: AximarMetadata;
}

export interface Notebook {
  nbformat: number;
  nbformat_minor: number;
  metadata: NotebookMetadata;
  cells: NotebookCell[];
}

export interface TemplateSummary {
  id: string;
  title: string;
  description: string;
  cell_count: number;
}

/** Extract the source text from a cell source (handles string or array). */
export function cellSourceText(source: string | string[]): string {
  return Array.isArray(source) ? source.join("") : source;
}
