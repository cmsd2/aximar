import { useState, useEffect, useCallback, useRef } from "react";
import ReactMarkdown from "react-markdown";
import remarkMath from "remark-math";
import rehypeKatex from "rehype-katex";
import { getFunctionDocs, getFunction, searchFunctions } from "../lib/catalog-client";
import type { MaximaFunction, SearchResult } from "../types/catalog";
import { convertFileSrc } from "@tauri-apps/api/core";

interface DocsPanelProps {
  open: boolean;
  functionName?: string;
  requestId: number;
  onClose: () => void;
}

export function DocsPanel({ open, functionName, requestId, onClose }: DocsPanelProps) {
  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<SearchResult[]>([]);
  const [currentFunction, setCurrentFunction] = useState<MaximaFunction | null>(null);
  const [currentDocs, setCurrentDocs] = useState<string | null>(null);
  const [loading, setLoading] = useState(false);
  const contentRef = useRef<HTMLDivElement>(null);
  const searchInputRef = useRef<HTMLInputElement>(null);

  // Use refs for history to avoid dependency cycles
  const historyRef = useRef<string[]>([]);
  const historyIndexRef = useRef(-1);
  const [, forceUpdate] = useState(0);

  // Track which request we've already handled
  const handledRequestRef = useRef(0);

  // Navigate to a function's docs — stable reference, no state deps
  const navigateTo = useCallback(async (name: string, addToHistory = true) => {
    setLoading(true);
    setSearchQuery("");
    setSearchResults([]);

    try {
      const [func, docs] = await Promise.all([
        getFunction(name),
        getFunctionDocs(name),
      ]);

      setCurrentFunction(func);
      setCurrentDocs(docs);

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
      return;
    }

    const timer = setTimeout(async () => {
      try {
        const results = await searchFunctions(searchQuery);
        setSearchResults(results.slice(0, 20));
      } catch {
        setSearchResults([]);
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

  return (
    <div className="docs-panel">
      <div className="docs-panel-header">
        <div className="docs-panel-nav">
          <button
            className="docs-nav-btn"
            onClick={goBack}
            disabled={historyIndex <= 0}
            title="Back"
          >
            &larr;
          </button>
          <button
            className="docs-nav-btn"
            onClick={goForward}
            disabled={historyIndex >= historyLength - 1}
            title="Forward"
          >
            &rarr;
          </button>
        </div>
        <input
          ref={searchInputRef}
          className="docs-search-input"
          type="text"
          placeholder="Search functions..."
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
            } else if (e.key === "Enter" && searchResults.length > 0) {
              navigateTo(searchResults[0].function.name);
            }
          }}
        />
        <button className="docs-close-btn" onClick={onClose} title="Close docs">
          &times;
        </button>
      </div>

      {searchQuery && searchResults.length > 0 && (
        <div className="docs-search-results">
          {searchResults.map((r) => (
            <button
              key={r.function.name}
              className="docs-search-result"
              onClick={() => navigateTo(r.function.name)}
            >
              <span className="docs-result-name">{r.function.name}</span>
              <span className="docs-result-sig">
                {r.function.signatures[0] || ""}
              </span>
            </button>
          ))}
        </div>
      )}

      <div className="docs-panel-content" ref={contentRef}>
        {loading ? (
          <div className="docs-loading">Loading...</div>
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

            {currentDocs ? (
              <div className="docs-markdown-body">
                <ReactMarkdown
                  remarkPlugins={[remarkMath]}
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
                      let imgSrc = src || "";
                      if (imgSrc.startsWith("figures/")) {
                        try {
                          imgSrc = convertFileSrc(imgSrc, "asset");
                        } catch {
                          // Fallback: keep original src
                        }
                      }
                      return <img {...props} src={imgSrc} alt={alt || ""} className="docs-figure" />;
                    },
                  }}
                >
                  {currentDocs}
                </ReactMarkdown>
              </div>
            ) : (
              <div className="docs-no-content">
                <p>{currentFunction.description}</p>
              </div>
            )}

            {currentFunction.see_also.length > 0 && (
              <div className="docs-see-also">
                <h3>See Also</h3>
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
            <p>Search for a function or click &ldquo;Docs&rdquo; on a hover tooltip to view documentation.</p>
          </div>
        )}
      </div>
    </div>
  );
}
