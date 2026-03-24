export interface Suggestion {
  label: string;
  template: string;
  description: string;
  /** When set, triggers a frontend action instead of Maxima evaluation. */
  action?: string;
  /** Where to insert the new cell: "before" or "after" (default). */
  position?: "before" | "after";
}
