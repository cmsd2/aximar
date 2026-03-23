import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { searchFunctions, listCategories } from "../lib/catalog-client";
import { MATH_SYMBOLS } from "../lib/math-symbols";
import { useNotebookStore } from "../store/notebookStore";
import { useLogStore } from "../store/logStore";
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
  const addLogEntry = useLogStore((s) => s.addEntry);

  // Load categories on mount
  useEffect(() => {
    listCategories()
      .then(setCategories)
      .catch((e) => addLogEntry("error", `Failed to load categories: ${e}`, "catalog"));
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Search on query change
  useEffect(() => {
    if (query.trim()) {
      const timer = setTimeout(() => {
        searchFunctions(query.trim())
          .then((r) => {
            setResults(r);
            setSelectedIndex(0);
          })
          .catch((e) => addLogEntry("error", `Catalog search failed: ${e}`, "catalog"));
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
      if (targetCellId) {
        // Get the current cell input length to position cursor inside parens
        const cells = useNotebookStore.getState().cells;
        const cell = cells.find((c) => c.id === targetCellId);
        const pos = cell ? cell.input.length - 1 : 0;
        useNotebookStore.getState().setPendingCursorMove({ cellId: targetCellId, pos });
      }
    },
    [insertTextInActiveCell, addCell, setActiveCellId, onClose, activeCellId]
  );

  const insertSymbol = useCallback(
    (unicode: string) => {
      if (activeCellId) {
        insertTextInActiveCell(unicode);
      } else {
        const newId = addCell();
        setActiveCellId(newId);
        useNotebookStore.getState().insertTextInActiveCell(unicode);
      }
      onClose();
    },
    [insertTextInActiveCell, addCell, setActiveCellId, onClose, activeCellId]
  );

  // Display mode: "categories" | "categoryFunctions" | "search" | "symbols"
  const isSymbolMode = query.startsWith("\\");
  const isSearchMode = !isSymbolMode && query.trim().length > 0;
  const isCategorySelected = !isSearchMode && !isSymbolMode && selectedCategory !== null;
  const isCategoryList = !isSearchMode && !isSymbolMode && selectedCategory === null;

  const filteredSymbols = useMemo(() => {
    if (!isSymbolMode) return [];
    const prefix = query.slice(1); // strip leading backslash
    if (!prefix) return MATH_SYMBOLS;
    return MATH_SYMBOLS.filter((s) => s.latex.startsWith(prefix));
  }, [isSymbolMode, query]);

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
      if (category === "__symbols__") {
        setQuery("\\");
        setSelectedIndex(0);
        return;
      }
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

  const navigableCount = isSymbolMode
    ? filteredSymbols.length
    : isCategoryList
      ? categories.length + 1 // +1 for Symbols entry
      : displayItems.length;

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
        if (isSymbolMode) {
          insertSymbol(filteredSymbols[selectedIndex].unicode);
        } else if (isCategoryList) {
          if (selectedIndex < categories.length) {
            selectCategory(categories[selectedIndex].category);
          } else {
            selectCategory("__symbols__");
          }
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
      isSymbolMode,
      categories,
      displayItems,
      filteredSymbols,
      selectedIndex,
      insertFunction,
      insertSymbol,
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

  const placeholder = isSymbolMode
    ? "Type symbol name (e.g. \\alpha, \\leq, \\nabla)..."
    : isCategorySelected
      ? `Filter ${selectedCategoryLabel}... (Backspace to go back)`
      : "Search functions (or type \\ for symbols)...";

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
          {isSymbolMode &&
            filteredSymbols.map((s, i) => (
              <div
                key={`${s.latex}-${i}`}
                className={`palette-item ${i === selectedIndex ? "selected" : ""}`}
                onClick={() => insertSymbol(s.unicode)}
                onMouseEnter={() => setSelectedIndex(i)}
              >
                <div className="palette-symbol-row">
                  <span className="palette-symbol-char">{s.unicode}</span>
                  <span className="palette-symbol-name">\{s.latex}</span>
                  <span className="palette-symbol-maxima">{s.maxima}</span>
                </div>
              </div>
            ))}
          {isSymbolMode && filteredSymbols.length === 0 && (
            <div className="palette-empty">No matching symbols</div>
          )}
          {isCategoryList && (<>
            {categories.map((g, i) => (
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
            <div
              key="__symbols__"
              className={`palette-category-item ${categories.length === selectedIndex ? "selected" : ""}`}
              onClick={() => selectCategory("__symbols__")}
              onMouseEnter={() => setSelectedIndex(categories.length)}
            >
              <span className="palette-category-item-label">Symbols</span>
              <span className="palette-category-item-count">
                {MATH_SYMBOLS.length}
              </span>
            </div>
          </>)}
          {!isCategoryList && !isSymbolMode && displayItems.length === 0 && query.trim() && (
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
