import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { searchFunctions, listCategories } from "../lib/catalog-client";
import { useNotebookStore } from "../store/notebookStore";
import type { SearchResult, CategoryGroup } from "../types/catalog";

interface CommandPaletteProps {
  onClose: () => void;
  onViewDocs?: (name: string) => void;
  initialQuery?: string;
}

export function CommandPalette({ onClose, onViewDocs, initialQuery }: CommandPaletteProps) {
  const [query, setQuery] = useState(initialQuery ?? "");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [categories, setCategories] = useState<CategoryGroup[]>([]);
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const insertTextInActiveCell = useNotebookStore(
    (s) => s.insertTextInActiveCell
  );
  const addCell = useNotebookStore((s) => s.addCell);
  const setActiveCellId = useNotebookStore((s) => s.setActiveCellId);

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
      const text = `${name}()`;
      let targetCellId = activeCellId;
      if (targetCellId) {
        insertTextInActiveCell(text);
      } else {
        const newId = addCell();
        setActiveCellId(newId);
        // addCell creates an empty cell; insert text via store update
        useNotebookStore.getState().insertTextInActiveCell(text);
        targetCellId = newId;
      }
      onClose();
      const focusCellId = targetCellId;
      requestAnimationFrame(() => {
        if (!focusCellId) return;
        const textarea = document.querySelector<HTMLTextAreaElement>(
          `textarea[data-cell-id="${focusCellId}"]`
        );
        if (textarea) {
          textarea.focus();
          const pos = textarea.value.length - 1;
          textarea.setSelectionRange(pos, pos);
        }
      });
    },
    [insertTextInActiveCell, addCell, setActiveCellId, onClose, activeCellId]
  );

  // Display mode: "categories" | "categoryFunctions" | "search"
  const isSearchMode = query.trim().length > 0;
  const isCategorySelected = !isSearchMode && selectedCategory !== null;
  const isCategoryList = !isSearchMode && selectedCategory === null;

  const categoryFunctionItems: SearchResult[] = useMemo(() => {
    if (!isCategorySelected) return [];
    const group = categories.find((g) => g.category === selectedCategory);
    if (!group) return [];
    return group.functions.map((f) => ({ function: f, score: 0 }));
  }, [categories, selectedCategory, isCategorySelected]);

  const displayItems = isSearchMode
    ? results
    : isCategorySelected
      ? categoryFunctionItems
      : [];

  const selectCategory = useCallback(
    (category: string) => {
      setSelectedCategory(category);
      setSelectedIndex(0);
    },
    []
  );

  const viewDocs = useCallback(
    (name: string) => {
      onClose();
      onViewDocs?.(name);
    },
    [onClose, onViewDocs]
  );

  const navigableCount = isCategoryList ? categories.length : displayItems.length;

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "ArrowDown") {
        e.preventDefault();
        setSelectedIndex((i) => Math.min(i + 1, navigableCount - 1));
      } else if (e.key === "ArrowUp") {
        e.preventDefault();
        setSelectedIndex((i) => Math.max(i - 1, 0));
      } else if (
        e.key === "Enter" &&
        (e.metaKey || e.ctrlKey) &&
        !isCategoryList &&
        displayItems.length > 0 &&
        onViewDocs
      ) {
        e.preventDefault();
        viewDocs(displayItems[selectedIndex].function.name);
      } else if (e.key === "Enter" && navigableCount > 0) {
        e.preventDefault();
        if (isCategoryList) {
          selectCategory(categories[selectedIndex].category);
        } else {
          insertFunction(displayItems[selectedIndex].function.name);
        }
      } else if (
        e.key === "Backspace" &&
        query === "" &&
        isCategorySelected
      ) {
        e.preventDefault();
        setSelectedCategory(null);
        setSelectedIndex(0);
      } else if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    },
    [
      navigableCount,
      isCategoryList,
      isCategorySelected,
      categories,
      displayItems,
      selectedIndex,
      insertFunction,
      viewDocs,
      onViewDocs,
      selectCategory,
      query,
      onClose,
    ]
  );

  // Close on Escape regardless of focus
  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onClose]);

  // Scroll selected item into view
  useEffect(() => {
    const list = listRef.current;
    if (!list) return;
    const item = list.children[selectedIndex] as HTMLElement;
    if (item) {
      item.scrollIntoView({ block: "nearest" });
    }
  }, [selectedIndex]);

  const selectedCategoryLabel = isCategorySelected
    ? categories.find((g) => g.category === selectedCategory)?.label ??
      selectedCategory
    : null;

  const placeholder = isCategorySelected
    ? `Filter ${selectedCategoryLabel}... (Backspace to go back)`
    : "Search Maxima functions...";

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
          placeholder={placeholder}
          autoFocus
        />
        <div className="palette-results" ref={listRef} onMouseDown={(e) => e.preventDefault()}>
          {isCategoryList &&
            categories.map((g, i) => (
              <div
                key={g.category}
                className={`palette-category-item ${i === selectedIndex ? "selected" : ""}`}
                onClick={() => selectCategory(g.category)}
                onMouseEnter={() => setSelectedIndex(i)}
              >
                <span className="palette-category-item-label">{g.label}</span>
                <span className="palette-category-item-count">
                  {g.functions.length}
                </span>
              </div>
            ))}
          {!isCategoryList && displayItems.length === 0 && query.trim() && (
            <div className="palette-empty">No functions found</div>
          )}
          {!isCategoryList &&
            displayItems.map((item, i) => {
              const f = item.function;
              return (
                <div key={`${f.name}-${i}`}>
                  <div
                    className={`palette-item ${i === selectedIndex ? "selected" : ""}`}
                    onClick={() => insertFunction(f.name)}
                    onMouseEnter={() => setSelectedIndex(i)}
                  >
                    <div className="palette-item-main">
                      <div className="palette-item-name">{f.name}</div>
                      <div className="palette-item-sig">
                        {f.signatures[0] || ""}
                      </div>
                      <div className="palette-item-desc">{f.description}</div>
                    </div>
                    {onViewDocs && (
                      <button
                        className="palette-item-docs"
                        onClick={(e) => {
                          e.stopPropagation();
                          viewDocs(f.name);
                        }}
                        title="View docs"
                      >
                        ?
                      </button>
                    )}
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
            <kbd>Enter</kbd> {isCategoryList ? "select" : "insert"}
          </span>
          {!isCategoryList && onViewDocs && (
            <span>
              <kbd>&#8984;Enter</kbd> docs
            </span>
          )}
          {isCategorySelected && (
            <span>
              <kbd>Backspace</kbd> back
            </span>
          )}
          <span>
            <kbd>Esc</kbd> close
          </span>
        </div>
      </div>
    </div>
  );
}
