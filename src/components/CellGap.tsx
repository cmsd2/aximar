import { useCallback, useEffect, useRef, useState } from "react";
import { useLingui } from "@lingui/react/macro";
import { getActiveTabState, useActiveTab, useNotebookStore, type InsertionPoint } from "../store/notebookStore";
import { IconPlus } from "./ui/icons";
import { fromGap, goTo, insertAt, typeFor } from "../lib/cell-caret";

/**
 * The gap between two cells, or above the first.  Clicking it, or an
 * arrow key from a cell's edge, puts a caret there; typing then starts a
 * cell with that text (a Markdown cell if it starts with #).  In card
 * style a + in the left margin opens a menu of cell types; bracket style
 * keeps its + Code and + Markdown buttons.
 */
export function CellGap({ after }: { after: string | null }) {
  const { t } = useLingui();
  const cellStyle = useNotebookStore((s) => s.cellStyle);
  const point = useActiveTab().insertionPoint;
  const setInsertionPoint = useNotebookStore((s) => s.setInsertionPoint);
  const active = point !== null && point.after === after;
  const here: InsertionPoint = { after };
  const [menuOpen, setMenuOpen] = useState(false);

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

  return (
    <div
      className={`cell-gap${cellStyle === "bracket" ? " insert-bar" : ""}${active ? " has-caret" : ""}${menuOpen ? " menu-open" : ""}`}
      onMouseDown={place}
      data-testid={`gap-${after ?? "top"}`}
    >
      {cellStyle === "bracket" ? (
        <>
          <div className="insert-bar-line" />
          <div className="insert-bar-buttons">
            <button className="insert-bar-btn" onClick={() => add("code")}>
              {t({
                message: "+ Code",
                comment: "Button in the bar between cells: inserts a code cell there. max-length:12",
              })}
            </button>
            <button className="insert-bar-btn" onClick={() => add("markdown")}>
              {t({
                message: "+ Markdown",
                comment: "Button in the bar between cells: inserts a markdown cell there. max-length:14",
              })}
            </button>
          </div>
          <div className="insert-bar-line" />
        </>
      ) : (
        <>
          <div className="cell-gap-line" />
          <button
            className="cell-gap-plus"
            aria-haspopup="menu"
            aria-expanded={menuOpen}
            title={t({
              message: "Insert a cell",
              comment: "Tooltip of the + in the margin between two cells: opens a menu of cell types to insert there",
            })}
            onClick={() => setMenuOpen((open) => !open)}
          >
            <IconPlus size={14} stroke={1.75} aria-hidden="true" />
          </button>
          {menuOpen && <GapMenu onPick={add} onClose={() => setMenuOpen(false)} />}
        </>
      )}
      {active && <Caret point={here} />}
    </div>
  );
}

/** The + menu: the cell types to insert */
function GapMenu({ onPick, onClose }: { onPick: (type: "code" | "markdown") => void; onClose: () => void }) {
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

  return (
    <div className="cell-gap-menu" role="menu" ref={ref} onKeyDown={keys}>
      <button role="menuitem" onClick={() => onPick("code")}>
        {t({ message: "Code", comment: "Item in the menu of the + between cells: inserts a Maxima code cell. max-length:16" })}
      </button>
      <button role="menuitem" onClick={() => onPick("markdown")}>
        {t({ message: "Markdown", comment: "Item in the menu of the + between cells: inserts a Markdown text cell. max-length:16" })}
      </button>
    </div>
  );
}

/**
 * The caret: a line in the gap, and a hidden input that takes the
 * keyboard.  The first text typed starts a cell; Enter starts an empty
 * code cell; the arrow keys move into the cells beside the gap; Escape,
 * or focus going elsewhere, removes the caret.
 */
function Caret({ point }: { point: InsertionPoint }) {
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

