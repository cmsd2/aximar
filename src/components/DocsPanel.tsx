import React, { useState, useEffect, useCallback, useRef } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import remarkMath from "remark-math";
import rehypeKatex from "rehype-katex";
import katex from "katex";
import { Trans, useLingui } from "@lingui/react/macro";
import { plural } from "@lingui/core/macro";
import { listen } from "@tauri-apps/api/event";
import { docsLanguages, getFunctionDocs, getFunction, searchFunctions, getPackage, packageForFunction, searchPackageFunctions, searchPackages } from "../lib/catalog-client";
import { setConfig } from "../lib/config-client";
import type { DocsLanguages, MaximaFunction, SearchResult, PackageInfo, PackageFunctionSearchResult, PackageSearchResult } from "../types/catalog";
import { DocsLanguageMenu } from "./docs/DocsLanguageMenu";
import { DocsLanguageNotes } from "./docs/DocsLanguageNotes";

/**
 * Convert single-line `$$...$$` to multi-line format required by remark-math v6.
 * (remark-math treats display math like code fences — delimiters must be on own lines.)
 */
function preprocessDocsMath(md: string): string {
  return md.replace(/^\$\$(.+)\$\$$/gm, "$$$$\n$1\n$$$$");
}

interface DocsPanelProps {
  open: boolean;
  functionName?: string;
  requestId: number;
  onClose: () => void;
}

export function DocsPanel({ open, functionName, requestId, onClose }: DocsPanelProps) {
  const { t } = useLingui();
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [pkgFuncResults, setPkgFuncResults] = useState<PackageFunctionSearchResult[]>([]);
  const [pkgResults, setPkgResults] = useState<PackageSearchResult[]>([]);
  const [currentFunction, setCurrentFunction] = useState<MaximaFunction | null>(null);
  const [currentDocs, setCurrentDocs] = useState<string | null>(null);
  const [currentPackage, setCurrentPackage] = useState<PackageInfo | null>(null);
  const [stubFunctionName, setStubFunctionName] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const [docsInfo, setDocsInfo] = useState<DocsLanguages | null>(null);
  const contentRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);

  // Use refs for history to avoid dependency cycles
  const historyRef = useRef<string[]>([]);
  const historyIndexRef = useRef(-1);
  const [, forceUpdate] = useState(0);

  // Track which request we've already handled
  const handledRequestRef = useRef(0);

  // Navigate to a function's docs or package page — stable reference, no state deps
  const navigateTo = useCallback(async (name: string, addToHistory = true) => {
    setLoading(true);
    setSearchQuery("");
    setSearchResults([]);
    setPkgFuncResults([]);
    setPkgResults([]);

    try {
      if (name.startsWith("pkg:")) {
        // Package page
        const pkgName = name.slice(4);
        const pkg = await getPackage(pkgName);
        setCurrentPackage(pkg);
        setCurrentFunction(null);
        setCurrentDocs(null);
        setStubFunctionName(null);
      } else {
        // Function page
        const [func, docs] = await Promise.all([
          getFunction(name),
          getFunctionDocs(name),
        ]);

        if (func) {
          setCurrentFunction(func);
          setCurrentDocs(docs);
          setCurrentPackage(null);
          setStubFunctionName(null);
        } else {
          // Not in catalog — check if it's a package function
          const pkgName = await packageForFunction(name);
          if (pkgName) {
            const pkg = await getPackage(pkgName);
            setCurrentPackage(pkg);
            setStubFunctionName(name);
            setCurrentFunction(null);
            setCurrentDocs(null);
          } else {
            // Unknown function — show empty state
            setCurrentFunction(null);
            setCurrentDocs(null);
            setCurrentPackage(null);
            setStubFunctionName(null);
          }
        }
      }

      if (addToHistory) {
        const newHistory = historyRef.current.slice(0, historyIndexRef.current + 1);
        newHistory.push(name);
        historyRef.current = newHistory;
        historyIndexRef.current = newHistory.length - 1;
      }

      // Force re-render so back/forward buttons update
      forceUpdate((n) => n + 1);

      // Scroll to top
      if (contentRef.current) {
        contentRef.current.scrollTop = 0;
      }
    } catch (err) {
      console.error("Failed to load docs for", name, err);
    } finally {
      setLoading(false);
    }
  }, []);

  // The help's languages; again when the app's language changes, which the
  // help follows unless a language is chosen for it
  const loadDocsInfo = useCallback(() => {
    docsLanguages()
      .then(setDocsInfo)
      .catch(() => setDocsInfo(null));
  }, []);

  useEffect(() => {
    loadDocsInfo();
    const unlisten = listen("language-changed", () => {
      loadDocsInfo();
      const page = historyRef.current[historyIndexRef.current];
      if (page) void navigateTo(page, false);
    });
    return () => {
      void unlisten.then((f) => f());
    };
  }, [loadDocsInfo, navigateTo]);

  /** Choose the help's language, and show the page again in it */
  const chooseDocsLanguage = useCallback(
    async (language: string | null) => {
      await setConfig({ docs_language: language });
      loadDocsInfo();
      const page = historyRef.current[historyIndexRef.current];
      if (page) await navigateTo(page, false);
    },
    [loadDocsInfo, navigateTo],
  );

  // Load docs when a new request arrives (requestId bumps on each click)
  useEffect(() => {
    if (open && functionName && requestId > 0 && requestId !== handledRequestRef.current) {
      handledRequestRef.current = requestId;
      navigateTo(functionName);
    }
  }, [open, functionName, requestId, navigateTo]);

  // Search as user types
  useEffect(() => {
    if (!searchQuery.trim()) {
      setSearchResults([]);
      setPkgFuncResults([]);
      setPkgResults([]);
      return;
    }

    const timer = setTimeout(async () => {
      try {
        const [results, pkgFuncs, pkgs] = await Promise.all([
          searchFunctions(searchQuery).catch(() => [] as SearchResult[]),
          searchPackageFunctions(searchQuery).catch(() => [] as PackageFunctionSearchResult[]),
          searchPackages(searchQuery).catch(() => [] as PackageSearchResult[]),
        ]);
        setSearchResults(results.slice(0, 20));
        setPkgFuncResults(pkgFuncs);
        setPkgResults(pkgs.slice(0, 5));
      } catch {
        setSearchResults([]);
        setPkgFuncResults([]);
        setPkgResults([]);
      }
    }, 150);

    return () => clearTimeout(timer);
  }, [searchQuery]);

  const goBack = useCallback(() => {
    if (historyIndexRef.current > 0) {
      historyIndexRef.current -= 1;
      navigateTo(historyRef.current[historyIndexRef.current], false);
    }
  }, [navigateTo]);

  const goForward = useCallback(() => {
    if (historyIndexRef.current < historyRef.current.length - 1) {
      historyIndexRef.current += 1;
      navigateTo(historyRef.current[historyIndexRef.current], false);
    }
  }, [navigateTo]);

  if (!open) return null;

  const historyIndex = historyIndexRef.current;
  const historyLength = historyRef.current.length;
  const packageName = currentPackage?.name ?? "";
  const functionCount = currentPackage?.functions.length ?? 0;
  const packageFunctionCount = (count: number) =>
    t({
      message: plural(count, { one: "# function", other: "# functions" }),
      comment: "Docs panel search results: how many functions a Maxima package provides",
    });

  return (
    <div className="docs-panel">
      <div className="docs-panel-header">
        <div className="docs-panel-nav">
          <button
            className="docs-nav-btn"
            onClick={goBack}
            disabled={historyIndex <= 0}
            title={t({
              message: "Back",
              comment: "Docs panel: tooltip of the arrow button that goes back to the previous page",
            })}
          >
            &larr;
          </button>
          <button
            className="docs-nav-btn"
            onClick={goForward}
            disabled={historyIndex >= historyLength - 1}
            title={t({
              message: "Forward",
              comment: "Docs panel: tooltip of the arrow button that goes forward to the next page",
            })}
          >
            &rarr;
          </button>
        </div>
        <input
          ref={searchInputRef}
          className="docs-search-input"
          type="text"
          placeholder={t({
            message: "Search functions...",
            comment: "Docs panel: placeholder of the search box at the top of the panel",
          })}
          value={searchQuery}
          onChange={(e) => setSearchQuery(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Escape") {
              if (searchQuery) {
                setSearchQuery("");
                setSearchResults([]);
              } else {
                onClose();
              }
            } else if (e.key === "Enter" && (searchResults.length > 0 || pkgFuncResults.length > 0 || pkgResults.length > 0)) {
              // Navigate to the highest-scoring result (normalized across backends)
              const normMax = (arr: { score: number }[]) => arr.reduce((m, r) => Math.max(m, r.score), 0);
              const norm = (score: number, max: number) => max > 0 ? score / max : 0;
              const maxCat = normMax(searchResults);
              const maxPkgF = normMax(pkgFuncResults);
              const maxPkg = normMax(pkgResults);
              type Scored = { nav: string; score: number };
              const all: Scored[] = [
                ...searchResults.map((r) => ({ nav: r.function.name, score: norm(r.score, maxCat) })),
                ...pkgFuncResults.map((r) => ({ nav: r.function_name, score: norm(r.score, maxPkgF) })),
                ...pkgResults.map((r) => ({ nav: `pkg:${r.package.name}`, score: norm(r.score, maxPkg) })),
              ];
              all.sort((a, b) => b.score - a.score);
              if (all.length > 0) navigateTo(all[0].nav);
            }
          }}
        />
        {docsInfo && <DocsLanguageMenu info={docsInfo} onChange={(l) => void chooseDocsLanguage(l)} />}
        <button
          className="docs-close-btn"
          onClick={onClose}
          title={t({
            message: "Close docs",
            comment: "Docs panel: tooltip of the x button that closes the panel",
          })}
        >
          &times;
        </button>
      </div>

      {searchQuery && (searchResults.length > 0 || pkgFuncResults.length > 0 || pkgResults.length > 0) && (
        <div className="docs-search-results">
          {(() => {
            type DocResult =
              | { kind: "catalog"; name: string; sig: string; nav: string }
              | { kind: "pkgFunc"; name: string; pkg: string; nav: string }
              | { kind: "package"; name: string; count: number; nav: string };

            // Normalize scores to [0, 1] within each result set so different
            // backends (BM25 vs manual scoring) are comparable when merged.
            const normalize = <T extends { score: number }>(arr: T[]): (T & { norm: number })[] => {
              const max = arr.reduce((m, r) => Math.max(m, r.score), 0);
              return arr.map((r) => ({ ...r, norm: max > 0 ? r.score / max : 0 }));
            };

            type ScoredDocResult = DocResult & { score: number };
            const items: ScoredDocResult[] = [
              ...normalize(searchResults).map((r): ScoredDocResult => ({
                kind: "catalog", name: r.function.name,
                sig: r.function.signatures[0] || "", nav: r.function.name,
                score: r.norm,
              })),
              ...normalize(pkgFuncResults).map((r): ScoredDocResult => ({
                kind: "pkgFunc", name: r.function_name,
                pkg: r.package_name, nav: r.function_name,
                score: r.norm,
              })),
              ...normalize(pkgResults).map((r): ScoredDocResult => ({
                kind: "package", name: r.package.name,
                count: r.package.functions.length, nav: `pkg:${r.package.name}`,
                score: r.norm,
              })),
            ];

            // Sort by normalized relevance score (descending)
            items.sort((a, b) => b.score - a.score);

            return items.map((item) => (
              <button
                key={`${item.kind}-${item.name}`}
                className="docs-search-result"
                onClick={() => navigateTo(item.nav)}
              >
                <span className="docs-result-name">
                  {item.kind !== "catalog" && (
                    <span className="palette-package-badge">
                      {t({
                        message: "pkg",
                        comment:
                          "Docs panel search results: short badge before a name that marks a Maxima package or a function from one. max-length:4",
                      })}
                    </span>
                  )}
                  {item.name}
                </span>
                <span className="docs-result-sig">
                  {item.kind === "catalog" ? item.sig
                    : item.kind === "pkgFunc" ? item.pkg
                    : packageFunctionCount(item.count)}
                </span>
              </button>
            ));
          })()}
        </div>
      )}

      <div className="docs-panel-content" ref={contentRef}>
        {loading ? (
          <div className="docs-loading">
            {t({
              message: "Loading...",
              comment: "Docs panel: shown while a documentation page loads",
            })}
          </div>
        ) : currentPackage && stubFunctionName ? (
          <>
            <div className="docs-function-header">
              <h2 className="docs-function-name">{stubFunctionName}</h2>
              <span className="docs-category-badge">
                {t({
                  message: "Package Function",
                  comment:
                    "Docs panel: badge beside a function name that marks it as provided by a loadable package",
                })}
              </span>
            </div>

            <div className="docs-no-content">
              <p>
                <Trans comment="Docs panel: on the page of a function from a package; packageName is the package name, a link to its page">
                  Provided by the{" "}
                  <a
                    className="docs-fn-link"
                    href="#"
                    onClick={(e) => {
                      e.preventDefault();
                      navigateTo(`pkg:${packageName}`);
                    }}
                  >
                    {packageName}
                  </a>{" "}
                  package.
                </Trans>
              </p>
            </div>

            {currentPackage.signatures?.[stubFunctionName] && (
              <div className="docs-signatures">
                <code className="docs-signature">{currentPackage.signatures[stubFunctionName]}</code>
              </div>
            )}

            {!currentPackage.builtin && (
              <div className="docs-signatures">
                <code className="docs-signature">load("{currentPackage.name}")$</code>
              </div>
            )}

            {currentPackage.functions.length > 0 && (
              <div className="docs-see-also">
                <h3>
                  {t({
                    message: "Related Functions",
                    comment: "Docs panel: heading above the other functions in the same package",
                  })}
                </h3>
                <div className="docs-see-also-links">
                  {currentPackage.functions
                    .filter((name) => name !== stubFunctionName)
                    .map((name) => (
                      <a
                        key={name}
                        className="docs-fn-link"
                        href="#"
                        onClick={(e) => {
                          e.preventDefault();
                          navigateTo(name);
                        }}
                      >
                        {currentPackage.signatures?.[name] || name}
                      </a>
                    ))}
                </div>
              </div>
            )}
          </>
        ) : currentPackage ? (
          <>
            <div className="docs-function-header">
              <h2 className="docs-function-name">{currentPackage.name}</h2>
              <span className="docs-category-badge">
                {t({
                  message: "Package",
                  comment: "Docs panel: badge beside a name that marks the page as a Maxima package",
                })}
              </span>
            </div>

            {!currentPackage.builtin && (
              <div className="docs-signatures">
                <code className="docs-signature">load("{currentPackage.name}")$</code>
              </div>
            )}

            <div className="docs-no-content">
              <p>{currentPackage.description}</p>
            </div>

            {currentPackage.functions.length > 0 && (
              <div className="docs-see-also">
                <h3>
                  {t({
                    message: `Functions (${functionCount})`,
                    comment:
                      "Docs panel: heading above the list of functions on a package page; functionCount is how many there are",
                  })}
                </h3>
                <div className="docs-see-also-links">
                  {currentPackage.functions.map((name) => (
                    <a
                      key={name}
                      className="docs-fn-link"
                      href="#"
                      onClick={(e) => {
                        e.preventDefault();
                        navigateTo(name);
                      }}
                    >
                      {currentPackage.signatures?.[name] || name}
                    </a>
                  ))}
                </div>
              </div>
            )}
          </>
        ) : currentFunction ? (
          <>
            <div className="docs-function-header">
              <h2 className="docs-function-name">{currentFunction.name}</h2>
              <span className="docs-category-badge">
                {currentFunction.category}
              </span>
            </div>

            {currentFunction.signatures.length > 0 && (
              <div className="docs-signatures">
                {currentFunction.signatures.map((sig, i) => (
                  <code key={i} className="docs-signature">{sig}</code>
                ))}
              </div>
            )}

            <DocsLanguageNotes fn={currentFunction} info={docsInfo} />

            {currentDocs ? (
              <div className="docs-markdown-body">
                <ReactMarkdown
                  remarkPlugins={[remarkGfm, remarkMath]}
                  rehypePlugins={[rehypeKatex]}
                  components={{
                    a: ({ href, children, ...props }) => {
                      if (href?.startsWith("fn:")) {
                        const fnName = href.slice(3);
                        return (
                          <a
                            {...props}
                            className="docs-fn-link"
                            href="#"
                            onClick={(e) => {
                              e.preventDefault();
                              navigateTo(fnName);
                            }}
                          >
                            {children}
                          </a>
                        );
                      }
                      return (
                        <a {...props} href={href} target="_blank" rel="noopener noreferrer">
                          {children}
                        </a>
                      );
                    },
                    img: ({ src, alt, ...props }) => {
                      // figures/ paths are served from public/figures/ by Vite
                      const imgSrc = src?.startsWith("figures/") ? `/${src}` : src || "";
                      return <img {...props} src={imgSrc} alt={alt || ""} className="docs-figure" />;
                    },
                    pre: ({ children, ...props }) => {
                      // Render $$...$$ lines as KaTeX inside maxima code blocks
                      const child = React.Children.toArray(children)[0];
                      if (!React.isValidElement(child)) return <pre {...props}>{children}</pre>;
                      const codeProps = child.props as { className?: string; children?: React.ReactNode };
                      const className = codeProps.className || "";
                      const content = String(codeProps.children || "");

                      if (!className.includes("language-maxima") || !content.includes("$$")) {
                        return <pre {...props}>{children}</pre>;
                      }

                      // Split on $$...$$ regions (possibly spanning multiple lines)
                      const parts = content.split(/\$\$([\s\S]*?)\$\$/);
                      const elements: React.ReactNode[] = [];
                      for (let i = 0; i < parts.length; i++) {
                        if (i % 2 === 0) {
                          // Code text (between math regions)
                          if (parts[i]) elements.push(parts[i]);
                        } else {
                          // Math capture group
                          const tex = parts[i].replace(/\n\s*/g, " ").trim();
                          const html = katex.renderToString(tex, {
                            displayMode: true,
                            throwOnError: false,
                          });
                          elements.push(
                            <span key={i} dangerouslySetInnerHTML={{ __html: html }} />,
                          );
                        }
                      }
                      return (
                        <pre {...props}>
                          <code className={className}>{elements}</code>
                        </pre>
                      );
                    },
                  }}
                >
                  {preprocessDocsMath(currentDocs)}
                </ReactMarkdown>
              </div>
            ) : (
              <div className="docs-no-content docs-markdown-body">
                <ReactMarkdown
                  remarkPlugins={[remarkGfm, remarkMath]}
                  rehypePlugins={[rehypeKatex]}
                >
                  {currentFunction.description}
                </ReactMarkdown>
              </div>
            )}

            {currentFunction.see_also.length > 0 && (
              <div className="docs-see-also">
                <h3>
                  {t({
                    message: "See Also",
                    comment: "Docs panel: heading above links to related functions",
                  })}
                </h3>
                <div className="docs-see-also-links">
                  {currentFunction.see_also.map((name) => (
                    <a
                      key={name}
                      className="docs-fn-link"
                      href="#"
                      onClick={(e) => {
                        e.preventDefault();
                        navigateTo(name);
                      }}
                    >
                      {name}
                    </a>
                  ))}
                </div>
              </div>
            )}
          </>
        ) : (
          <div className="docs-empty">
            <p>
              <Trans comment="Docs panel: shown when no page is open; Docs is the name of the button on a function's hover tooltip">
                Search for a function or click &ldquo;Docs&rdquo; on a hover tooltip to view documentation.
              </Trans>
            </p>
          </div>
        )}
      </div>
    </div>
  );
}
