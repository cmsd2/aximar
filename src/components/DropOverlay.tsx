import { plural } from "@lingui/core/macro";
import { useLingui } from "@lingui/react/macro";
import type { DragState } from "../hooks/useFileDrop";

/**
 * Shown over the window while files are dragged onto it: how many
 * notebooks will open, or that the files aren't notebooks.
 */
export function DropOverlay({ drag }: { drag: DragState | null }) {
  const { t } = useLingui();
  if (!drag) return null;
  const { notebooks, others } = drag;
  const accepted = notebooks > 0;

  const heading = accepted
    ? t({
        message: plural(notebooks, { one: "Drop to open # notebook", other: "Drop to open # notebooks" }),
        comment: "Shown over the window while notebook files are dragged onto it. # is how many notebooks will open",
      })
    : t({
        message: plural(others, { one: "This file isn't a notebook", other: "These files aren't notebooks" }),
        comment: "Shown over the window while files that Aximar can't open are dragged onto it",
      });

  const note =
    !accepted || others > 0
      ? t({
          message: "Aximar opens .macnb and .ipynb notebooks.",
          comment: "Under the drop message when some dragged files aren't notebooks. .macnb and .ipynb are file extensions and stay as they are",
        })
      : null;

  return (
    <div className={`drop-overlay${accepted ? "" : " drop-overlay-refused"}`} aria-live="polite">
      <div className="drop-overlay-card">
        <div className="drop-overlay-heading">{heading}</div>
        {note && <div className="drop-overlay-note">{note}</div>}
      </div>
    </div>
  );
}
