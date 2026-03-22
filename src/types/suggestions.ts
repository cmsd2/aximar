export interface Suggestion {
  label: string;
  template: string;
  description: string;
  /** When set, triggers a frontend action instead of Maxima evaluation. */
  action?: string;
}
