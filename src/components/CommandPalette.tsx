import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { searchFunctions, listCategories } from "../lib/catalog-client";
import { useNotebookStore } from "../store/notebookStore";
import type { SearchResult, CategoryGroup } from "../types/catalog";

interface CommandPaletteProps {
  onClose: () => void;
  initialQuery?: string;
}

export function CommandPalette({ onClose, initialQuery }: CommandPaletteProps) {
  const [query, setQuery] = useState(initialQuery ?? "");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [categories, setCategories] = useState<CategoryGroup[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const insertTextInActiveCell = useNotebookStore(
    (s) => s.insertTextInActiveCell
  );

  // Load categories on mount
  useEffect(() => {
    listCategories()
      .then(setCategories)
      .catch(() => {});
  }, []);

  // Search on query change
  useEffect(() => {
    if (query.trim()) {
      const timer = setTimeout(() => {
        searchFunctions(query.trim())
          .then((r) => {
            setResults(r);
            setSelectedIndex(0);
          })
          .catch(() => {});
      }, 80);
      return () => clearTimeout(timer);
    } else {
      setResults([]);
      setSelectedIndex(0);
    }
  }, [query]);

  const activeCellId = useNotebookStore((s) => s.activeCellId);

  const insertFunction = useCallback(
    (name: string) => {
      insertTextInActiveCell(`${name}()`);
      onClose();
      requestAnimationFrame(() => {
        if (!activeCellId) return;
        const textarea = document.querySelector<HTMLTextAreaElement>(
          `textarea[data-cell-id="${activeCellId}"]`
        );
        if (textarea) {
          textarea.focus();
          const pos = textarea.value.length - 1;
          textarea.setSelectionRange(pos, pos);
        }
      });
    },
    [insertTextInActiveCell, onClose, activeCellId]
  );

  // Cap displayed results to avoid freezing the UI
  const MAX_DISPLAY = 50;
  const categoryItems: SearchResult[] = useMemo(() => {
    const items: SearchResult[] = [];
    for (const g of categories) {
      for (const f of g.functions) {
        items.push({ function: f, score: 0 });
        if (items.length >= MAX_DISPLAY) return items;
      }
    }
    return items;
  }, [categories]);

  const displayItems = query.trim()
    ? results.slice(0, MAX_DISPLAY)
    : categoryItems;

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((i) => Math.min(i + 1, displayItems.length - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((i) => Math.max(i - 1, 0));
      } else if (e.key === "Enter" && displayItems.length > 0) {
        e.preventDefault();
        insertFunction(displayItems[selectedIndex].function.name);
      } else if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    },
    [displayItems, selectedIndex, insertFunction, onClose]
  );

  // Scroll selected item into view
  useEffect(() => {
    const list = listRef.current;
    if (!list) return;
    const item = list.children[selectedIndex] as HTMLElement;
    if (item) {
      item.scrollIntoView({ block: "nearest" });
    }
  }, [selectedIndex]);

  let lastCategory = "";

  return (
    <div className="palette-overlay" onClick={onClose}>
      <div className="palette-modal" onClick={(e) => e.stopPropagation()}>
        <input
          ref={inputRef}
          className="palette-input"
          type="text"
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Search Maxima functions..."
          autoFocus
        />
        <div className="palette-results" ref={listRef}>
          {displayItems.length === 0 && query.trim() && (
            <div className="palette-empty">No functions found</div>
          )}
          {displayItems.map((item, i) => {
            const f = item.function;
            let header: string | null = null;

            // Show category headers in browse mode
            if (!query.trim()) {
              const cat = f.category;
              if (cat !== lastCategory) {
                lastCategory = cat;
                header = categories.find((g) => g.category === cat)?.label || cat;
              }
            }

            return (
              <div key={`${f.name}-${i}`}>
                {header && (
                  <div className="palette-category-header">{header}</div>
                )}
                <div
                  className={`palette-item ${i === selectedIndex ? "selected" : ""}`}
                  onClick={() => insertFunction(f.name)}
                  onMouseEnter={() => setSelectedIndex(i)}
                >
                  <div className="palette-item-name">{f.name}</div>
                  <div className="palette-item-sig">
                    {f.signatures[0] || ""}
                  </div>
                  <div className="palette-item-desc">{f.description}</div>
                </div>
              </div>
            );
          })}
        </div>
        <div className="palette-footer">
          <span>
            <kbd>&uarr;</kbd> <kbd>&darr;</kbd> navigate
          </span>
          <span>
            <kbd>Enter</kbd> insert
          </span>
          <span>
            <kbd>Esc</kbd> close
          </span>
        </div>
      </div>
    </div>
  );
}
