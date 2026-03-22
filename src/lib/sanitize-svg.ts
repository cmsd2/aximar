/** Dangerous elements that must be removed from SVG content. */
const DANGEROUS_ELEMENTS = new Set([
  "script",
  "foreignobject",
  "iframe",
  "object",
  "embed",
]);

/** Schemes that are not allowed in href attributes. */
const DANGEROUS_HREF_RE = /^\s*(javascript|data):/i;

/**
 * Sanitize an SVG string by removing dangerous elements and attributes.
 *
 * Uses the browser's built-in DOMParser — no extra dependencies needed.
 */
export function sanitizeSvg(raw: string): string {
  const parser = new DOMParser();
  const doc = parser.parseFromString(raw, "image/svg+xml");

  // If parsing failed, return empty string rather than risk passing through raw content
  const parseError = doc.querySelector("parsererror");
  if (parseError) {
    console.warn("[sanitize-svg] Failed to parse SVG");
    return "";
  }

  stripDangerous(doc.documentElement);

  const serializer = new XMLSerializer();
  return serializer.serializeToString(doc.documentElement);
}

function stripDangerous(el: Element): void {
  // Remove dangerous child elements (iterate in reverse so removal is safe)
  const children = Array.from(el.children);
  for (const child of children) {
    if (DANGEROUS_ELEMENTS.has(child.localName.toLowerCase())) {
      child.remove();
      continue;
    }
    stripDangerous(child);
  }

  // Remove event handler attributes (on*)
  const attrs = Array.from(el.attributes);
  for (const attr of attrs) {
    const name = attr.name.toLowerCase();
    if (name.startsWith("on")) {
      el.removeAttribute(attr.name);
    }
    // Remove dangerous href schemes
    if (
      (name === "href" || name === "xlink:href") &&
      DANGEROUS_HREF_RE.test(attr.value)
    ) {
      el.removeAttribute(attr.name);
    }
  }
}
