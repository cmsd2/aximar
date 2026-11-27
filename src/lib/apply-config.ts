// Settings that change how the window looks: applied at startup with the
// whole config, and by the Settings window with each change.
import { applyPrintMargins, markdownFontStack, type AppConfig } from "./config-client";
import type { AutocompleteMode, CellStyle, Theme } from "../store/notebookStore";

/** What else a change reaches besides the document */
export interface ConfigTargets {
  setTheme: (theme: Theme) => void;
  setCellStyle: (style: CellStyle) => void;
  setAutocompleteMode: (mode: AutocompleteMode) => void;
  setShowSuggestions: (show: boolean) => void;
  setVariablesOpen: (open: boolean) => void;
}

/**
 * Apply `updates` to the document and `targets`.  `next` is the whole
 * config after the change, for settings applied together, such as the four
 * print margins.
 */
export function applyConfig(updates: Partial<AppConfig>, next: AppConfig, targets: ConfigTargets): void {
  const style = document.documentElement.style;
  if (updates.theme) targets.setTheme(updates.theme as Theme);
  if (updates.cell_style) {
    targets.setCellStyle(updates.cell_style as CellStyle);
    document.documentElement.dataset.cellStyle = updates.cell_style;
  }
  if (updates.autocomplete_mode) targets.setAutocompleteMode(updates.autocomplete_mode as AutocompleteMode);
  if (updates.show_suggestions !== undefined) targets.setShowSuggestions(updates.show_suggestions);
  if (updates.markdown_font) style.setProperty("--font-family-markdown", markdownFontStack(updates.markdown_font));
  if (updates.markdown_indent) {
    style.setProperty("--markdown-indent", updates.markdown_indent === "aligned" ? "var(--gutter-width)" : "16px");
  }
  if (updates.variables_open !== undefined) targets.setVariablesOpen(updates.variables_open);
  if (updates.font_size !== undefined) style.setProperty("--font-size-mono", `${updates.font_size}px`);
  if (updates.print_font_size !== undefined) {
    style.setProperty("--print-font-size", `${updates.print_font_size}px`);
    style.setProperty("--print-font-size-mono", `${updates.print_font_size - 1}px`);
  }
  if (
    updates.print_margin_top !== undefined ||
    updates.print_margin_bottom !== undefined ||
    updates.print_margin_left !== undefined ||
    updates.print_margin_right !== undefined
  ) {
    applyPrintMargins(next.print_margin_top, next.print_margin_bottom, next.print_margin_left, next.print_margin_right);
  }
}
