import { useEffect, useRef, useCallback } from "react";
import { useFindStore } from "../store/findStore";
import { useNotebookStore } from "../store/notebookStore";
import type { FindMatch } from "../store/findStore";

export function FindBar() {
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

  const cells = useNotebookStore((s) => s.cells);

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
    const newMatches: FindMatch[] = [];
    const q = caseSensitive ? query : query.toLowerCase();
    for (const cell of cells) {
      const text = caseSensitive ? cell.input : cell.input.toLowerCase();
      let pos = 0;
      while (pos < text.length) {
        const idx = text.indexOf(q, pos);
        if (idx === -1) break;
        newMatches.push({ cellId: cell.id, start: idx, end: idx + query.length });
        pos = idx + 1;
      }
    }
    setMatches(newMatches);
  }, [query, cells, caseSensitive, setMatches]);

  // Navigate to current match
  useEffect(() => {
    if (matches.length === 0) return;
    const match = matches[currentMatchIndex];
    if (!match) return;

    const { setActiveCellId } = useNotebookStore.getState();
    setActiveCellId(match.cellId);

    requestAnimationFrame(() => {
      const textarea = document.querySelector<HTMLTextAreaElement>(
        `[data-cell-id="${match.cellId}"]`
      );
      if (textarea) {
        textarea.focus();
        textarea.setSelectionRange(match.start, match.end);
        // Scroll cell into view
        const cellEl = textarea.closest(".cell");
        cellEl?.scrollIntoView({ block: "nearest", behavior: "smooth" });
      }
    });
  }, [currentMatchIndex, matches]);

  const handleReplace = useCallback(() => {
    if (matches.length === 0) return;
    const match = matches[currentMatchIndex];
    if (!match) return;

    const { forceInputSnapshot, updateCellInput, cells: currentCells } = useNotebookStore.getState();
    const cell = currentCells.find((c) => c.id === match.cellId);
    if (!cell) return;

    forceInputSnapshot();
    const newText = cell.input.slice(0, match.start) + replacement + cell.input.slice(match.end);
    updateCellInput(match.cellId, newText);
  }, [matches, currentMatchIndex, replacement]);

  const handleReplaceAll = useCallback(() => {
    if (matches.length === 0) return;

    const { forceInputSnapshot, updateCellInput, cells: currentCells } = useNotebookStore.getState();
    forceInputSnapshot();

    // Group matches by cell
    const byCellId = new Map<string, FindMatch[]>();
    for (const m of matches) {
      const arr = byCellId.get(m.cellId) ?? [];
      arr.push(m);
      byCellId.set(m.cellId, arr);
    }

    for (const [cellId, cellMatches] of byCellId) {
      const cell = currentCells.find((c) => c.id === cellId);
      if (!cell) continue;
      // Process in reverse order so indices stay valid
      let text = cell.input;
      for (let i = cellMatches.length - 1; i >= 0; i--) {
        const m = cellMatches[i];
        text = text.slice(0, m.start) + replacement + text.slice(m.end);
      }
      updateCellInput(cellId, text);
    }
  }, [matches, replacement]);

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

  const matchText =
    matches.length > 0
      ? `${currentMatchIndex + 1} of ${matches.length}`
      : query
        ? "No results"
        : "";

  return (
    <div className="find-bar">
      <div className="find-bar-row">
        <input
          ref={findInputRef}
          className="find-bar-input"
          type="text"
          placeholder="Find..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={handleFindKeyDown}
        />
        <span className="find-bar-count">{matchText}</span>
        <button
          className={`find-bar-btn find-bar-case${caseSensitive ? " active" : ""}`}
          onClick={toggleCaseSensitive}
          title="Match Case"
        >
          Aa
        </button>
        <button
          className="find-bar-btn"
          onClick={goToPrevMatch}
          disabled={matches.length === 0}
          title="Previous Match (Shift+Enter)"
        >
          &#9650;
        </button>
        <button
          className="find-bar-btn"
          onClick={goToNextMatch}
          disabled={matches.length === 0}
          title="Next Match (Enter)"
        >
          &#9660;
        </button>
        <button
          className={`find-bar-btn find-bar-replace-toggle${replaceVisible ? " active" : ""}`}
          onClick={toggleReplaceVisible}
          title="Toggle Replace"
        >
          &#8596;
        </button>
        <button
          className="find-bar-btn find-bar-close"
          onClick={close}
          title="Close (Escape)"
        >
          &times;
        </button>
      </div>
      {replaceVisible && (
        <div className="find-bar-row">
          <input
            className="find-bar-input"
            type="text"
            placeholder="Replace..."
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
            title="Replace"
          >
            Replace
          </button>
          <button
            className="find-bar-btn"
            onClick={handleReplaceAll}
            disabled={matches.length === 0}
            title="Replace All"
          >
            All
          </button>
        </div>
      )}
    </div>
  );
}
