import { hoverTooltip, type Tooltip } from "@codemirror/view";
import katex from "katex";
import { getFunction, packageForFunction, getPackage } from "./catalog-client";
import type { MaximaFunction, PackageInfo } from "../types/catalog";

type HoverData =
  | { kind: "catalog"; func: MaximaFunction }
  | { kind: "package"; funcName: string; pkg: PackageInfo }
  | null;

const hoverCache = new Map<string, HoverData>();

export function maximaHoverTooltip(onViewDocs?: (name: string) => void) {
  return hoverTooltip(
    async (view, pos): Promise<Tooltip | null> => {
      // Find the word at position
      const { from, to, text: word } = getWordAtPos(view.state.doc.toString(), pos);
      if (!word || word.length < 2) return null;

      // Check cache
      let data: HoverData | undefined = hoverCache.get(word);
      if (data === undefined) {
        try {
          const func = await getFunction(word);
          if (func) {
            data = { kind: "catalog", func };
          } else {
            // Fallback: check package functions
            const pkgName = await packageForFunction(word);
            if (pkgName) {
              const pkg = await getPackage(pkgName);
              if (pkg) {
                data = { kind: "package", funcName: word, pkg };
              } else {
                data = null;
              }
            } else {
              data = null;
            }
          }
          hoverCache.set(word, data);
        } catch {
          hoverCache.set(word, null);
          return null;
        }
      }

      if (!data) return null;

      return {
        pos: from,
        end: to,
        above: false,
        create() {
          if (data!.kind === "catalog") {
            return { dom: renderHoverTooltip(data!.func, onViewDocs) };
          } else {
            return { dom: renderPackageFuncTooltip(data!.funcName, data!.pkg, onViewDocs) };
          }
        },
      };
    },
    { hideOnChange: true, hoverTime: 150 }
  );
}

function getWordAtPos(
  text: string,
  pos: number
): { from: number; to: number; text: string } {
  let from = pos;
  let to = pos;

  // Expand left
  while (from > 0 && /[a-zA-Z_0-9]/.test(text[from - 1])) {
    from--;
  }
  // Expand right
  while (to < text.length && /[a-zA-Z_0-9]/.test(text[to])) {
    to++;
  }

  const word = text.slice(from, to);
  // Must start with letter or underscore
  if (!word || !/^[a-zA-Z_]/.test(word)) {
    return { from: pos, to: pos, text: "" };
  }

  return { from, to, text: word };
}

function renderHoverTooltip(
  func: MaximaFunction,
  onViewDocs?: (name: string) => void
): HTMLElement {
  const container = document.createElement("div");
  container.className = "hover-tooltip";

  // Signatures
  const sigDiv = document.createElement("div");
  sigDiv.className = "hover-tooltip-sig";
  const sigs = (func.signatures.length > 0 ? func.signatures : [func.name]).slice(0, 3);
  for (const sig of sigs) {
    const line = document.createElement("div");
    line.textContent = sig;
    sigDiv.appendChild(line);
  }
  if (func.signatures.length > 3) {
    const more = document.createElement("div");
    more.className = "hover-tooltip-more";
    more.textContent = `+${func.signatures.length - 3} more`;
    sigDiv.appendChild(more);
  }
  container.appendChild(sigDiv);

  // Description (with inline KaTeX math)
  const descDiv = document.createElement("div");
  descDiv.className = "hover-tooltip-desc";
  renderMathText(descDiv, func.description);
  container.appendChild(descDiv);

  // Footer
  const footerDiv = document.createElement("div");
  footerDiv.className = "hover-tooltip-footer";

  const catSpan = document.createElement("span");
  catSpan.className = "hover-tooltip-category";
  catSpan.textContent = func.category;
  footerDiv.appendChild(catSpan);

  if (onViewDocs) {
    const docsBtn = document.createElement("button");
    docsBtn.className = "hover-tooltip-link";
    docsBtn.innerHTML = "Docs &rarr;";
    docsBtn.addEventListener("mousedown", (e) => {
      e.preventDefault();
      onViewDocs(func.name);
    });
    footerDiv.appendChild(docsBtn);
  }

  container.appendChild(footerDiv);

  return container;
}

function renderPackageFuncTooltip(
  funcName: string,
  pkg: PackageInfo,
  onViewDocs?: (name: string) => void
): HTMLElement {
  const container = document.createElement("div");
  container.className = "hover-tooltip";

  // Signature (or just name)
  const sigDiv = document.createElement("div");
  sigDiv.className = "hover-tooltip-sig";
  const line = document.createElement("div");
  line.textContent = pkg.signatures?.[funcName] || `${funcName}()`;
  sigDiv.appendChild(line);
  container.appendChild(sigDiv);

  // Description
  const descDiv = document.createElement("div");
  descDiv.className = "hover-tooltip-desc";
  descDiv.textContent = `requires load("${pkg.name}")`;
  container.appendChild(descDiv);

  // Footer
  const footerDiv = document.createElement("div");
  footerDiv.className = "hover-tooltip-footer";

  const catSpan = document.createElement("span");
  catSpan.className = "hover-tooltip-category";
  catSpan.textContent = `Package: ${pkg.name}`;
  footerDiv.appendChild(catSpan);

  if (onViewDocs) {
    const docsBtn = document.createElement("button");
    docsBtn.className = "hover-tooltip-link";
    docsBtn.innerHTML = "Docs &rarr;";
    docsBtn.addEventListener("mousedown", (e) => {
      e.preventDefault();
      onViewDocs(funcName);
    });
    footerDiv.appendChild(docsBtn);
  }

  container.appendChild(footerDiv);
  return container;
}

/** Render text with inline ($...$) and display ($$...$$) KaTeX math into a DOM element. */
function renderMathText(el: HTMLElement, text: string) {
  let i = 0;
  while (i < text.length) {
    // Display math $$...$$
    if (text[i] === "$" && text[i + 1] === "$") {
      const end = text.indexOf("$$", i + 2);
      if (end !== -1) {
        const span = document.createElement("span");
        try {
          span.innerHTML = katex.renderToString(text.slice(i + 2, end), {
            displayMode: true,
            throwOnError: false,
            trust: false,
          });
        } catch {
          span.textContent = text.slice(i, end + 2);
        }
        el.appendChild(span);
        i = end + 2;
        continue;
      }
    }
    // Inline math $...$
    if (text[i] === "$") {
      const end = text.indexOf("$", i + 1);
      if (end !== -1) {
        const span = document.createElement("span");
        try {
          span.innerHTML = katex.renderToString(text.slice(i + 1, end), {
            displayMode: false,
            throwOnError: false,
            trust: false,
          });
        } catch {
          span.textContent = text.slice(i, end + 1);
        }
        el.appendChild(span);
        i = end + 1;
        continue;
      }
    }
    // Plain text until next $
    const next = text.indexOf("$", i);
    const chunk = next === -1 ? text.slice(i) : text.slice(i, next);
    if (chunk) {
      el.appendChild(document.createTextNode(chunk));
    }
    i = next === -1 ? text.length : next;
  }
}
