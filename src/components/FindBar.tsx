import { useEffect, useRef, useCallback } from "react";
import { useLingui } from "@lingui/react/macro";
import { useFindStore } from "../store/findStore";
import { useActiveTab, useNotebookStore, getActiveTabState } from "../store/notebookStore";
import type { FindMatch } from "../store/findStore";
import { findMatches, replaceAllMatches } from "../lib/find-replace";

export function FindBar() {
  const { t } = useLingui();
  const isOpen = useFindStore((s) => s.isOpen);
  const replaceVisible = useFindStore((s) => s.replaceVisible);
  const query = useFindStore((s) => s.query);
  const replacement = useFindStore((s) => s.replacement);
  const caseSensitive = useFindStore((s) => s.caseSensitive);
  const matches = useFindStore((s) => s.matches);
  const currentMatchIndex = useFindStore((s) => s.currentMatchIndex);
  const close = useFindStore((s) => s.close);
  const setQuery = useFindStore((s) => s.setQuery);
  const setReplacement = useFindStore((s) => s.setReplacement);
  const toggleCaseSensitive = useFindStore((s) => s.toggleCaseSensitive);
  const toggleReplaceVisible = useFindStore((s) => s.toggleReplaceVisible);
  const setMatches = useFindStore((s) => s.setMatches);
  const goToNextMatch = useFindStore((s) => s.goToNextMatch);
  const goToPrevMatch = useFindStore((s) => s.goToPrevMatch);

  const cells = useActiveTab().cells;

  const findInputRef = useRef<HTMLInputElement>(null);

  // Focus input when opening
  useEffect(() => {
    if (isOpen) {
      requestAnimationFrame(() => findInputRef.current?.select());
    }
  }, [isOpen]);

  // Recompute matches when query/cells/caseSensitive change
  useEffect(() => {
    if (!query) {
      setMatches([]);
      return;
    }
    const newMatches: FindMatch[] = cells.flatMap((cell) =>
      findMatches(cell.input, query, caseSensitive).map((m) => ({ cellId: cell.id, ...m }))
    );
    setMatches(newMatches);
  }, [query, cells, caseSensitive, setMatches]);

  // Navigate to current match
  useEffect(() => {
    if (matches.length === 0) return;
    const match = matches[currentMatchIndex];
    if (!match) return;

    const { setActiveCellId } = useNotebookStore.getState();
    setActiveCellId(match.cellId);

    useFindStore.getState().setNavigateTo({
      cellId: match.cellId,
      start: match.start,
      end: match.end,
    });
    // Scroll cell into view
    requestAnimationFrame(() => {
      const cellEl = document.querySelector(`[data-cell-id="${match.cellId}"]`);
      const container = cellEl?.closest(".cell");
      container?.scrollIntoView({ block: "nearest", behavior: "smooth" });
    });
  }, [currentMatchIndex, matches]);

  const handleReplace = useCallback(() => {
    if (matches.length === 0) return;
    const match = matches[currentMatchIndex];
    if (!match) return;

    const { updateCellInput } = useNotebookStore.getState();
    const currentCells = getActiveTabState().cells;
    const cell = currentCells.find((c: { id: string }) => c.id === match.cellId);
    if (!cell) return;

    const newText = cell.input.slice(0, match.start) + replacement + cell.input.slice(match.end);
    updateCellInput(match.cellId, newText);
  }, [matches, currentMatchIndex, replacement]);

  const handleReplaceAll = useCallback(() => {
    if (matches.length === 0) return;

    const { updateCellInput } = useNotebookStore.getState();
    // Match against each cell's current text rather than the stored matches,
    // which may be stale
    for (const cell of getActiveTabState().cells) {
      const text = replaceAllMatches(cell.input, query, replacement, caseSensitive);
      if (text !== cell.input) {
        updateCellInput(cell.id, text);
      }
    }
  }, [matches.length, query, replacement, caseSensitive]);

  const handleFindKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Enter" && e.shiftKey) {
        e.preventDefault();
        goToPrevMatch();
      } else if (e.key === "Enter") {
        e.preventDefault();
        goToNextMatch();
      } else if (e.key === "Escape") {
        e.preventDefault();
        close();
      }
    },
    [goToNextMatch, goToPrevMatch, close]
  );

  if (!isOpen) return null;

  const current = currentMatchIndex + 1;
  const total = matches.length;
  const matchText =
    total > 0
      ? t({
          message: `${current} of ${total}`,
          comment:
            "Find bar: which match is selected; current is its position and total is the number of matches. max-length:12",
        })
      : query
        ? t({
            message: "No results",
            comment: "Find bar: shown beside the search box when the text appears in no cell. max-length:12",
          })
        : "";
  const replaceLabel = t({
    message: "Replace",
    comment: "Find bar: button that replaces the selected match with the replacement text",
  });

  return (
    <div className="find-bar">
      <div className="find-bar-row">
        <input
          ref={findInputRef}
          className="find-bar-input"
          type="text"
          placeholder={t({
            message: "Find...",
            comment: "Find bar: placeholder of the box for the text to find in the notebook's cells",
          })}
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={handleFindKeyDown}
        />
        <span className="find-bar-count">{matchText}</span>
        <button
          className={`find-bar-btn find-bar-case${caseSensitive ? " active" : ""}`}
          onClick={toggleCaseSensitive}
          title={t({
            message: "Match Case",
            comment: "Find bar: tooltip of the Aa button that makes the search case-sensitive",
          })}
        >
          Aa
        </button>
        <button
          className="find-bar-btn"
          onClick={goToPrevMatch}
          disabled={matches.length === 0}
          title={t({
            message: "Previous Match (Shift+Enter)",
            comment: "Find bar: tooltip of the up arrow button that selects the previous match; Shift+Enter is the shortcut",
          })}
        >
          &#9650;
        </button>
        <button
          className="find-bar-btn"
          onClick={goToNextMatch}
          disabled={matches.length === 0}
          title={t({
            message: "Next Match (Enter)",
            comment: "Find bar: tooltip of the down arrow button that selects the next match; Enter is the shortcut",
          })}
        >
          &#9660;
        </button>
        <button
          className={`find-bar-btn find-bar-replace-toggle${replaceVisible ? " active" : ""}`}
          onClick={toggleReplaceVisible}
          title={t({
            message: "Toggle Replace",
            comment: "Find bar: tooltip of the button that shows or hides the replace row",
          })}
        >
          &#8596;
        </button>
        <button
          className="find-bar-btn find-bar-close"
          onClick={close}
          title={t({
            message: "Close (Escape)",
            comment: "Find bar: tooltip of the x button that closes the find bar; Escape is the key",
          })}
        >
          &times;
        </button>
      </div>
      {replaceVisible && (
        <div className="find-bar-row">
          <input
            className="find-bar-input"
            type="text"
            placeholder={t({
              message: "Replace...",
              comment: "Find bar: placeholder of the box for the replacement text",
            })}
            value={replacement}
            onChange={(e) => setReplacement(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                e.preventDefault();
                close();
              }
            }}
          />
          <button
            className="find-bar-btn"
            onClick={handleReplace}
            disabled={matches.length === 0}
            title={replaceLabel}
          >
            {replaceLabel}
          </button>
          <button
            className="find-bar-btn"
            onClick={handleReplaceAll}
            disabled={matches.length === 0}
            title={t({
              message: "Replace All",
              comment: "Find bar: tooltip of the All button that replaces every match in every cell",
            })}
          >
            {t({
              message: "All",
              comment: "Find bar: button that replaces every match in every cell. max-length:8",
            })}
          </button>
        </div>
      )}
    </div>
  );
}
