import { t } from "@lingui/core/macro";

/**
 * A tab's title for display: its file name, or "Untitled" in the user's
 * language for a notebook that has no file.  The store keeps an empty
 * title for those, so a language switch renames them.
 */
export function displayTitle(title: string): string {
  return (
    title ||
    t({
      message: "Untitled",
      comment: "Tab and toolbar: the name shown for a notebook that has not been saved to a file",
    })
  );
}
