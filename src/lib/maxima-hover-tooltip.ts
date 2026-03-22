import { hoverTooltip, type Tooltip } from "@codemirror/view";
import { getFunction } from "./catalog-client";
import type { MaximaFunction } from "../types/catalog";

const hoverCache = new Map<string, MaximaFunction | null>();

export function maximaHoverTooltip(onViewDocs?: (name: string) => void) {
  return hoverTooltip(
    async (view, pos): Promise<Tooltip | null> => {
      // Find the word at position
      const { from, to, text: word } = getWordAtPos(view.state.doc.toString(), pos);
      if (!word || word.length < 2) return null;

      // Check cache
      let func: MaximaFunction | null | undefined = hoverCache.get(word);
      if (func === undefined) {
        try {
          func = await getFunction(word);
          hoverCache.set(word, func);
        } catch {
          hoverCache.set(word, null);
          return null;
        }
      }

      if (!func) return null;

      return {
        pos: from,
        end: to,
        above: false,
        create() {
          return { dom: renderHoverTooltip(func!, onViewDocs) };
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

  // Description
  const descDiv = document.createElement("div");
  descDiv.className = "hover-tooltip-desc";
  descDiv.textContent = func.description;
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
