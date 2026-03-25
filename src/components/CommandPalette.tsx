import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { searchFunctions, listCategories, searchPackages, searchPackageFunctions } from "../lib/catalog-client";
import { MATH_SYMBOLS } from "../lib/math-symbols";
import { useNotebookStore } from "../store/notebookStore";
import { useLogStore } from "../store/logStore";
import { nbAddCell } from "../lib/notebook-commands";
import type { SearchResult, CategoryGroup, PackageSearchResult, PackageFunctionSearchResult } from "../types/catalog";

interface CommandPaletteProps {
  onClose: () => void;
  onViewDocs?: (name: string) => void;
  initialQuery?: string;
}

export function CommandPalette({ onClose, onViewDocs, initialQuery }: CommandPaletteProps) {
  const [query, setQuery] = useState(initialQuery ?? "");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [categories, setCategories] = useState<CategoryGroup[]>([]);
  const [packageResults, setPackageResults] = useState<PackageSearchResult[]>([]);
  const [pkgFuncResults, setPkgFuncResults] = useState<PackageFunctionSearchResult[]>([]);
  const [selectedCategory, setSelectedCategory] = useState<string | null>(null);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  const insertTextInActiveCell = useNotebookStore(
    (s) => s.insertTextInActiveCell
  );
  const setActiveCellId = useNotebookStore((s) => s.setActiveCellId);
  const addLogEntry = useLogStore((s) => s.addEntry);

  // Load categories on mount
  useEffect(() => {
    listCategories()
      .then(setCategories)
      .catch((e) => addLogEntry("error", `Failed to load categories: ${e}`, "catalog"));
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Display mode flags (computed early so search effect can reference them)
  const isSymbolMode = query.startsWith("\\");
  const isSearchMode = !isSymbolMode && query.trim().length > 0;
  const isPackagesMode = !isSearchMode && !isSymbolMode && selectedCategory === "__packages__";
  const isCategorySelected = !isSearchMode && !isSymbolMode && !isPackagesMode && selectedCategory !== null;
  const isCategoryList = !isSearchMode && !isSymbolMode && selectedCategory === null;

  // Search on query change
  useEffect(() => {
    if (query.trim() && !isSymbolMode) {
      const timer = setTimeout(() => {
        const q = query.trim();
        Promise.all([
          searchFunctions(q).catch(() => [] as SearchResult[]),
          searchPackages(q).catch(() => [] as PackageSearchResult[]),
          searchPackageFunctions(q).catch(() => [] as PackageFunctionSearchResult[]),
        ]).then(([funcs, pkgs, pkgFuncs]) => {
          setResults(funcs);
          setPackageResults(pkgs);
          setPkgFuncResults(pkgFuncs);
          setSelectedIndex(0);
        });
      }, 80);
      return () => clearTimeout(timer);
    } else {
      setResults([]);
      setPackageResults([]);
      setPkgFuncResults([]);
      setSelectedIndex(0);
    }
  }, [query, isSymbolMode]);

  const activeCellId = useNotebookStore((s) => s.activeCellId);

  const insertFunction = useCallback(
    async (name: string) => {
      const text = `${name}()`;
      let targetCellId = activeCellId;
      if (targetCellId) {
        insertTextInActiveCell(text);
      } else {
        const result = await nbAddCell("code", text);
        targetCellId = result.cell_id;
        setActiveCellId(targetCellId);
      }
      onClose();
      if (targetCellId) {
        const cells = useNotebookStore.getState().cells;
        const cell = cells.find((c) => c.id === targetCellId);
        const pos = cell ? cell.input.length - 1 : 0;
        useNotebookStore.getState().setPendingCursorMove({ cellId: targetCellId, pos });
      }
    },
    [insertTextInActiveCell, setActiveCellId, onClose, activeCellId]
  );

  const insertPackageLoad = useCallback(
    async (packageName: string) => {
      const text = `load("${packageName}")$`;
      let targetCellId = activeCellId;
      if (targetCellId) {
        insertTextInActiveCell(text);
      } else {
        const result = await nbAddCell("code", text);
        targetCellId = result.cell_id;
        setActiveCellId(targetCellId);
      }
      onClose();
    },
    [insertTextInActiveCell, setActiveCellId, onClose, activeCellId]
  );

  const insertSymbol = useCallback(
    async (unicode: string) => {
      if (activeCellId) {
        insertTextInActiveCell(unicode);
      } else {
        const result = await nbAddCell("code", unicode);
        setActiveCellId(result.cell_id);
      }
      onClose();
    },
    [insertTextInActiveCell, setActiveCellId, onClose, activeCellId]
  );

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

  type SearchItem =
    | { kind: "function"; name: string; sig: string; desc: string }
    | { kind: "pkgFunc"; name: string; packageName: string; sig: string }
    | { kind: "package"; name: string; desc: string; funcCount: number };

  const searchItems: SearchItem[] = useMemo(() => {
    if (!isSearchMode) return [];
    const items: SearchItem[] = [
      ...packageResults.slice(0, 5).map((r): SearchItem => ({
        kind: "package", name: r.package.name,
        desc: r.package.description, funcCount: r.package.functions.length,
      })),
      ...pkgFuncResults.map((r): SearchItem => ({
        kind: "pkgFunc", name: r.function_name, packageName: r.package_name,
        sig: r.signature || "",
      })),
      ...results.map((r): SearchItem => ({
        kind: "function", name: r.function.name,
        sig: r.function.signatures[0] || "", desc: r.function.description,
      })),
    ];
    return items;
  }, [isSearchMode, results, pkgFuncResults, packageResults]);

  const displayItems = isCategorySelected
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

  // Load all packages when in packages mode
  const allPackages = useMemo(() => {
    if (!isPackagesMode) return [];
    return packageResults;
  }, [isPackagesMode, packageResults]);

  // When entering packages mode, search for all packages
  useEffect(() => {
    if (isPackagesMode && packageResults.length === 0) {
      searchPackages("").then(setPackageResults).catch(() => {});
    }
  }, [isPackagesMode]); // eslint-disable-line react-hooks/exhaustive-deps

  const navigableCount = isSymbolMode
    ? filteredSymbols.length
    : isSearchMode
      ? searchItems.length
      : isPackagesMode
        ? allPackages.length
        : isCategoryList
          ? categories.length + 2 // +1 for Symbols, +1 for Packages
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
        isSearchMode &&
        searchItems.length > 0 &&
        onViewDocs
      ) {
        e.preventDefault();
        const item = searchItems[selectedIndex];
        if (item.kind === "package") {
          viewDocs(`pkg:${item.name}`);
        } else {
          viewDocs(item.name);
        }
      } else if (
        e.key === "Enter" &&
        (e.metaKey || e.ctrlKey) &&
        !isCategoryList &&
        !isPackagesMode &&
        !isSearchMode &&
        displayItems.length > 0 &&
        onViewDocs
      ) {
        e.preventDefault();
        viewDocs(displayItems[selectedIndex].function.name);
      } else if (
        e.key === "Enter" &&
        (e.metaKey || e.ctrlKey) &&
        isPackagesMode &&
        allPackages.length > 0 &&
        onViewDocs
      ) {
        e.preventDefault();
        viewDocs(`pkg:${allPackages[selectedIndex].package.name}`);
      } else if (e.key === "Enter" && navigableCount > 0) {
        e.preventDefault();
        if (isSymbolMode) {
          insertSymbol(filteredSymbols[selectedIndex].unicode);
        } else if (isSearchMode) {
          const item = searchItems[selectedIndex];
          if (item.kind === "package") {
            insertPackageLoad(item.name);
          } else {
            insertFunction(item.name);
          }
        } else if (isPackagesMode) {
          insertPackageLoad(allPackages[selectedIndex].package.name);
        } else if (isCategoryList) {
          if (selectedIndex < categories.length) {
            selectCategory(categories[selectedIndex].category);
          } else if (selectedIndex === categories.length) {
            selectCategory("__packages__");
          } else {
            selectCategory("__symbols__");
          }
        } else {
          insertFunction(displayItems[selectedIndex].function.name);
        }
      } else if (
        e.key === "Backspace" &&
        query === "" &&
        (isCategorySelected || isPackagesMode)
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
      isSearchMode,
      isSymbolMode,
      isPackagesMode,
      categories,
      displayItems,
      searchItems,
      allPackages,
      filteredSymbols,
      selectedIndex,
      insertFunction,
      insertPackageLoad,
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
    : isPackagesMode
      ? "Browse packages... (Backspace to go back)"
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
              key="__packages__"
              className={`palette-category-item ${categories.length === selectedIndex ? "selected" : ""}`}
              onClick={() => selectCategory("__packages__")}
              onMouseEnter={() => setSelectedIndex(categories.length)}
            >
              <span className="palette-category-item-label">Packages</span>
              <span className="palette-category-item-count">
                load()
              </span>
            </div>
            <div
              key="__symbols__"
              className={`palette-category-item ${categories.length + 1 === selectedIndex ? "selected" : ""}`}
              onClick={() => selectCategory("__symbols__")}
              onMouseEnter={() => setSelectedIndex(categories.length + 1)}
            >
              <span className="palette-category-item-label">Symbols</span>
              <span className="palette-category-item-count">
                {MATH_SYMBOLS.length}
              </span>
            </div>
          </>)}
          {isPackagesMode && allPackages.map((item, i) => (
            <div key={`pkg-${item.package.name}-${i}`}>
              <div
                className={`palette-item ${i === selectedIndex ? "selected" : ""}`}
                onClick={() => insertPackageLoad(item.package.name)}
                onMouseEnter={() => setSelectedIndex(i)}
              >
                <div className="palette-item-main">
                  <div className="palette-item-name">{item.package.name}</div>
                  <div className="palette-item-desc">{item.package.description}</div>
                  <div className="palette-item-sig">
                    {item.package.functions.length} functions
                  </div>
                </div>
                {onViewDocs && (
                  <button
                    className="palette-item-docs"
                    onClick={(e) => {
                      e.stopPropagation();
                      viewDocs(`pkg:${item.package.name}`);
                    }}
                    title="View package docs"
                  >
                    ?
                  </button>
                )}
              </div>
            </div>
          ))}
          {isSearchMode && searchItems.map((item, i) => (
            <div key={`${item.kind}-${item.name}-${i}`}>
              <div
                className={`palette-item ${i === selectedIndex ? "selected" : ""}`}
                onClick={() =>
                  item.kind === "package"
                    ? insertPackageLoad(item.name)
                    : insertFunction(item.name)
                }
                onMouseEnter={() => setSelectedIndex(i)}
              >
                <div className="palette-item-main">
                  <div className="palette-item-name">
                    {item.kind !== "function" && (
                      <span className="palette-package-badge">pkg</span>
                    )}
                    {item.name}
                  </div>
                  {item.kind === "function" && item.sig && (
                    <div className="palette-item-sig">{item.sig}</div>
                  )}
                  {item.kind === "pkgFunc" && item.sig && (
                    <div className="palette-item-sig">{item.sig}</div>
                  )}
                  <div className="palette-item-desc">
                    {item.kind === "function"
                      ? item.desc
                      : item.kind === "pkgFunc"
                        ? `requires load("${item.packageName}")`
                        : item.desc}
                  </div>
                  {item.kind === "package" && (
                    <div className="palette-item-sig">
                      {item.funcCount} functions
                    </div>
                  )}
                </div>
                {onViewDocs && (
                  <button
                    className="palette-item-docs"
                    onClick={(e) => {
                      e.stopPropagation();
                      viewDocs(
                        item.kind === "package" ? `pkg:${item.name}` : item.name
                      );
                    }}
                    title="View docs"
                  >
                    ?
                  </button>
                )}
              </div>
            </div>
          ))}
          {isSearchMode && searchItems.length === 0 && query.trim() && (
            <div className="palette-empty">No functions or packages found</div>
          )}
          {!isCategoryList && !isPackagesMode && !isSearchMode &&
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
            <kbd>Enter</kbd> {isCategoryList ? "select" : isPackagesMode ? "load" : "insert"}
          </span>
          {!isCategoryList && onViewDocs && (
            <span>
              <kbd>&#8984;Enter</kbd> docs
            </span>
          )}
          {(isCategorySelected || isPackagesMode) && (
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
