import { useState, useEffect, useCallback, useRef, useMemo } from "react";
import { Trans, useLingui } from "@lingui/react/macro";
import { plural } from "@lingui/core/macro";
import { searchFunctions, listCategories, searchPackages, searchPackageFunctions } from "../lib/catalog-client";
import { MATH_SYMBOLS } from "../lib/math-symbols";
import { useActiveTab, useNotebookStore, getActiveTabState } from "../store/notebookStore";
import { useLogStore } from "../store/logStore";
import { nbAddCell } from "../lib/notebook-commands";
import type { SearchResult, CategoryGroup, PackageSearchResult, PackageFunctionSearchResult } from "../types/catalog";

interface CommandPaletteProps {
  onClose: () => void;
  onViewDocs?: (name: string) => void;
  initialQuery?: string;
}

export function CommandPalette({ onClose, onViewDocs, initialQuery }: CommandPaletteProps) {
  const { t } = useLingui();
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

  const activeCellId = useActiveTab().activeCellId;

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
        const cells = getActiveTabState().cells;
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
    | { kind: "function"; name: string; sig: string; desc: string; score: number }
    | { kind: "pkgFunc"; name: string; packageName: string; sig: string; score: number; builtin: boolean }
    | { kind: "package"; name: string; desc: string; funcCount: number; score: number; builtin: boolean };

  const searchItems: SearchItem[] = useMemo(() => {
    if (!isSearchMode) return [];

    // Normalize scores to [0, 1] within each result set so different
    // backends (BM25 vs manual scoring) are comparable when merged.
    const normalize = <T extends { score: number }>(arr: T[]): (T & { norm: number })[] => {
      const max = arr.reduce((m, r) => Math.max(m, r.score), 0);
      return arr.map((r) => ({ ...r, norm: max > 0 ? r.score / max : 0 }));
    };

    const normResults = normalize(results);
    const normPkgFuncs = normalize(pkgFuncResults);
    const normPkgs = normalize(packageResults.slice(0, 5));

    const items: SearchItem[] = [
      ...normPkgs.map((r): SearchItem => ({
        kind: "package", name: r.package.name,
        desc: r.package.description, funcCount: r.package.functions.length,
        score: r.norm, builtin: !!r.package.builtin,
      })),
      ...normPkgFuncs.map((r): SearchItem => {
        const pkg = packageResults.find((p) => p.package.name === r.package_name);
        return {
          kind: "pkgFunc", name: r.function_name, packageName: r.package_name,
          sig: r.signature || "", score: r.norm, builtin: !!pkg?.package.builtin,
        };
      }),
      ...normResults.map((r): SearchItem => ({
        kind: "function", name: r.function.name,
        sig: r.function.signatures[0] || "", desc: r.function.description,
        score: r.norm,
      })),
    ];
    // Sort by normalized relevance score (descending)
    items.sort((a, b) => b.score - a.score);
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
            if (item.builtin) {
              onClose(); // built-in packages don't need load()
            } else {
              insertPackageLoad(item.name);
            }
          } else {
            insertFunction(item.name);
          }
        } else if (isPackagesMode) {
          const pkg = allPackages[selectedIndex].package;
          if (pkg.builtin) {
            onClose();
          } else {
            insertPackageLoad(pkg.name);
          }
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

  const categoryLabel = selectedCategoryLabel;
  const placeholder = isSymbolMode
    ? t({
        message: "Type symbol name (e.g. \\alpha, \\leq, \\nabla)...",
        comment:
          "Command palette: placeholder of the search box while it lists math symbols; \\alpha and the others are LaTeX names and stay as they are",
      })
    : isPackagesMode
      ? t({
          message: "Browse packages... (Backspace to go back)",
          comment:
            "Command palette: placeholder of the search box while it lists Maxima packages; Backspace is the key",
        })
      : isCategorySelected
        ? t({
            message: `Filter ${categoryLabel}... (Backspace to go back)`,
            comment:
              "Command palette: placeholder of the search box while it lists one category of functions; categoryLabel is the category name, such as Calculus; Backspace is the key",
          })
        : t({
            message: "Search functions (or type \\ for symbols)...",
            comment:
              "Command palette: placeholder of the search box; typing a backslash switches to the list of math symbols",
          });

  const viewDocsTitle = t({
    message: "View docs",
    comment: "Command palette: tooltip of the ? button that opens a function's documentation",
  });

  const packageFunctionCount = (count: number) =>
    t({
      message: plural(count, { one: "# function", other: "# functions" }),
      comment: "Command palette: how many functions a Maxima package provides",
    });

  const requiresLoad = (packageName: string) =>
    t({
      message: `requires load("${packageName}")`,
      comment:
        "Command palette: under a function from a package that is not loaded yet; load(...) is Maxima code and stays as it is; packageName is the package name",
    });

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
            <div className="palette-empty">
              {t({
                message: "No matching symbols",
                comment: "Command palette: shown when no math symbol matches what the user typed",
              })}
            </div>
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
              <span className="palette-category-item-label">
                {t({
                  message: "Packages",
                  comment: "Command palette: list entry that opens the list of loadable Maxima packages",
                })}
              </span>
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
              <span className="palette-category-item-label">
                {t({
                  message: "Symbols",
                  comment: "Command palette: list entry that opens the list of math symbols, such as Greek letters",
                })}
              </span>
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
                    {packageFunctionCount(item.package.functions.length)}
                  </div>
                </div>
                {onViewDocs && (
                  <button
                    className="palette-item-docs"
                    onClick={(e) => {
                      e.stopPropagation();
                      viewDocs(`pkg:${item.package.name}`);
                    }}
                    title={t({
                      message: "View package docs",
                      comment: "Command palette: tooltip of the ? button that opens a package's documentation",
                    })}
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
                      <span className="palette-package-badge">
                        {t({
                          message: "pkg",
                          comment:
                            "Command palette: short badge before a name that marks a Maxima package or a function from one. max-length:4",
                        })}
                      </span>
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
                        ? (item.builtin ? item.packageName : requiresLoad(item.packageName))
                        : item.desc}
                  </div>
                  {item.kind === "package" && (
                    <div className="palette-item-sig">
                      {packageFunctionCount(item.funcCount)}
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
                    title={viewDocsTitle}
                  >
                    ?
                  </button>
                )}
              </div>
            </div>
          ))}
          {isSearchMode && searchItems.length === 0 && query.trim() && (
            <div className="palette-empty">
              {t({
                message: "No functions or packages found",
                comment: "Command palette: shown when a search finds no Maxima function or package",
              })}
            </div>
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
                        title={viewDocsTitle}
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
            <Trans comment="Command palette footer: the arrow keys move through the list">
              <kbd>&uarr;</kbd> <kbd>&darr;</kbd> navigate
            </Trans>
          </span>
          <span>
            {isCategoryList ? (
              <Trans comment="Command palette footer: Enter opens the selected category">
                <kbd>Enter</kbd> select
              </Trans>
            ) : isPackagesMode ? (
              <Trans comment="Command palette footer: Enter inserts a load() call for the selected package">
                <kbd>Enter</kbd> load
              </Trans>
            ) : (
              <Trans comment="Command palette footer: Enter inserts the selected function or symbol into the cell">
                <kbd>Enter</kbd> insert
              </Trans>
            )}
          </span>
          {!isCategoryList && onViewDocs && (
            <span>
              <Trans comment="Command palette footer: Cmd+Enter opens the documentation of the selected item">
                <kbd>&#8984;Enter</kbd> docs
              </Trans>
            </span>
          )}
          {(isCategorySelected || isPackagesMode) && (
            <span>
              <Trans comment="Command palette footer: Backspace goes back to the list of categories">
                <kbd>Backspace</kbd> back
              </Trans>
            </span>
          )}
          <span>
            <Trans comment="Command palette footer: Esc closes the palette">
              <kbd>Esc</kbd> close
            </Trans>
          </span>
        </div>
      </div>
    </div>
  );
}
