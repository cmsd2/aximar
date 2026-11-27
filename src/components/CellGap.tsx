import { useCallback, useEffect, useRef, useState } from "react";
import { useLingui } from "@lingui/react/macro";
import { getActiveTabState, useActiveTab, useNotebookStore, type InsertionPoint } from "../store/notebookStore";
import { IconArrowRight, IconPlus } from "./ui/icons";
import { fromGap, goTo, insertAt, typeFor } from "../lib/cell-caret";
import { useCellSuggestions } from "../hooks/useCellSuggestions";
import type { Suggestion } from "../types/suggestions";
import type { Cell } from "../types/notebook";

/**
 * The gap between two cells, or above the first.  Clicking it, or an
 * arrow key from a cell's edge, puts a caret there; typing then starts a
 * cell with that text (a Markdown cell if it starts with #).  A + at the
 * line's left end opens a menu: a code or Markdown cell, and the
 * suggestions for the result above, such as "Simplify", which insert a
 * cell here and run it.
 */
export function CellGap({ after }: { after: string | null }) {
  const { t } = useLingui();
  const cellStyle = useNotebookStore((s) => s.cellStyle);
  const point = useActiveTab().insertionPoint;
  const setInsertionPoint = useNotebookStore((s) => s.setInsertionPoint);
  const active = point !== null && point.after === after;
  const here: InsertionPoint = { after };
  const [menuOpen, setMenuOpen] = useState(false);
  const cells = useActiveTab().cells;
  const showSuggestions = useNotebookStore((s) => s.showSuggestions);
  const above = after === null ? undefined : cells.find((c) => c.id === after);
  const { suggestions, choose } = useCellSuggestions(above?.cellType === "code" ? above : undefined, showSuggestions);

  const add = useCallback(
    (cellType: "code" | "markdown") => {
      setMenuOpen(false);
      void insertAt({ after }, cellType);
    },
    [after],
  );

  const place = (e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest("button, [role=menu]")) return;
    // Keep the browser from moving focus to the page, which would take it
    // from the caret's input as soon as it mounts
    e.preventDefault();
    setInsertionPoint(here);
  };

  const suggest = (suggestion: Suggestion) => {
    setMenuOpen(false);
    useNotebookStore.getState().setInsertionPoint(null);
    choose(suggestion);
  };

  return (
    <div
      className={`cell-gap${cellStyle === "bracket" ? " insert-bar" : ""}${active ? " has-caret" : ""}${menuOpen ? " menu-open" : ""}`}
      onMouseDown={place}
      data-testid={`gap-${after ?? "top"}`}
    >
      <div className="cell-gap-line" />
      <button
        className="cell-gap-plus"
        aria-haspopup="menu"
        aria-expanded={menuOpen}
        title={
          suggestions.length > 0
            ? t({
                message: "Insert a cell, or a next step for the result above",
                comment: "Tooltip of the + between two cells when the result above has suggested next steps, such as Simplify",
              })
            : t({
                message: "Insert a cell",
                comment: "Tooltip of the + in the margin between two cells: opens a menu of cell types to insert there",
              })
        }
        onClick={() => setMenuOpen((open) => !open)}
      >
        <IconPlus size={14} stroke={1.75} aria-hidden="true" />
      </button>
      {menuOpen && (
        <GapMenu
          onPick={add}
          onClose={() => setMenuOpen(false)}
          source={above}
          suggestions={suggestions}
          onSuggest={suggest}
        />
      )}
      {active && <Caret point={here} onMenu={() => setMenuOpen(true)} />}
    </div>
  );
}

/**
 * The + menu: the cell types to insert, then the next steps for the
 * result above, under a heading that names it
 */
function GapMenu({
  onPick,
  onClose,
  source,
  suggestions,
  onSuggest,
}: {
  onPick: (type: "code" | "markdown") => void;
  onClose: () => void;
  source: Cell | undefined;
  suggestions: Suggestion[];
  onSuggest: (s: Suggestion) => void;
}) {
  const { t } = useLingui();
  const ref = useRef<HTMLDivElement>(null);

  useEffect(() => {
    ref.current?.querySelector("button")?.focus();
    const away = (e: MouseEvent) => {
      if (!ref.current?.parentElement?.contains(e.target as Node)) onClose();
    };
    document.addEventListener("mousedown", away);
    return () => document.removeEventListener("mousedown", away);
  }, [onClose]);

  const keys = (e: React.KeyboardEvent) => {
    const items = Array.from(ref.current?.querySelectorAll("button") ?? []);
    const i = items.indexOf(document.activeElement as HTMLButtonElement);
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    } else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      e.preventDefault();
      const next = (i + (e.key === "ArrowDown" ? 1 : items.length - 1)) % items.length;
      items[next]?.focus();
    }
  };

  const run = source?.output?.executionCount ?? null;
  return (
    <div className="cell-gap-menu context-menu" role="menu" ref={ref} onKeyDown={keys}>
      <button role="menuitem" onClick={() => onPick("code")}>
        <span>{t({ message: "Code", comment: "Item in the menu of the + between cells: inserts a Maxima code cell. max-length:16" })}</span>
        <span className="context-menu-key">⏎</span>
      </button>
      <button role="menuitem" onClick={() => onPick("markdown")}>
        <span>
          {t({ message: "Markdown", comment: "Item in the menu of the + between cells: inserts a Markdown text cell. max-length:16" })}
        </span>
        <span className="context-menu-key">#</span>
      </button>
      {suggestions.length > 0 && (
        <>
          <div className="cell-gap-menu-heading" role="presentation">
            {run !== null
              ? t({
                  message: `From Out [${run}]`,
                  comment:
                    "Heading in the + menu between cells, above the suggested next steps for the result above, such as Simplify. run is that result's run number, as in its Out [n] label. max-length:20",
                })
              : t({
                  message: "From the result above",
                  comment: "Heading in the + menu between cells, above the suggested next steps for the result above. max-length:24",
                })}
          </div>
          {suggestions.map((s) => (
            <button
              key={s.action ?? s.template}
              role="menuitem"
              className="cell-gap-suggestion"
              title={s.description}
              onClick={() => onSuggest(s)}
            >
              <span>
                <IconArrowRight size={13} stroke={1.75} aria-hidden="true" /> {s.label}
              </span>
            </button>
          ))}
        </>
      )}
    </div>
  );
}

/**
 * The caret: a line in the gap, and a hidden input that takes the
 * keyboard.  The first text typed starts a cell; Enter starts an empty
 * code cell; Tab opens the + menu; the arrow keys move into the cells
 * beside the gap; Escape,
 * or focus going elsewhere, removes the caret.
 */
function Caret({ point, onMenu }: { point: InsertionPoint; onMenu: () => void }) {
  const { t } = useLingui();
  const input = useRef<HTMLTextAreaElement>(null);
  const started = useRef(false);
  const composing = useRef(false);

  useEffect(() => {
    input.current?.focus({ preventScroll: false });
    input.current?.scrollIntoView({ block: "nearest" });
  }, []);

  const start = (text: string, cellType = typeFor(text)) => {
    if (started.current) return;
    started.current = true;
    void insertAt(point, cellType, text, () => input.current?.value ?? text);
  };

  const keys = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (composing.current || started.current) return;
    const plain = !e.shiftKey && !e.altKey && !e.metaKey && !e.ctrlKey;
    if (e.key === "Escape") {
      e.preventDefault();
      useNotebookStore.getState().setInsertionPoint(null);
    } else if (e.key === "Tab" && plain) {
      // Tab goes into the + menu, as the next thing to focus
      e.preventDefault();
      onMenu();
    } else if (e.key === "Enter" && plain) {
      e.preventDefault();
      start("", "code");
    } else if ((e.key === "ArrowUp" || e.key === "ArrowDown") && plain) {
      e.preventDefault();
      const target = fromGap(getActiveTabState().cells, point, e.key === "ArrowUp" ? "up" : "down");
      if (target) goTo(target);
    }
  };

  return (
    <>
      <div className="cell-gap-caret" />
      <textarea
        ref={input}
        className="cell-gap-input"
        rows={1}
        aria-label={t({
          message: "New cell: type to start a code cell, or # for a Markdown heading",
          comment: "Screen reader label of the caret between two cells, where typing starts a new cell. # starts a Markdown heading and stays as it is",
        })}
        onKeyDown={keys}
        onCompositionStart={() => (composing.current = true)}
        onCompositionEnd={(e) => {
          composing.current = false;
          if (e.data) start(e.currentTarget.value || e.data);
        }}
        onInput={(e) => {
          const text = e.currentTarget.value;
          if (!composing.current && text) start(text);
        }}
        onBlur={() => {
          if (!started.current) useNotebookStore.getState().setInsertionPoint(null);
        }}
      />
    </>
  );
}

