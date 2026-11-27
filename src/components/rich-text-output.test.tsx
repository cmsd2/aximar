// @vitest-environment happy-dom
import { describe, expect, it } from "vitest";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { RichTextOutput } from "./RichTextOutput";

/** The output as the window renders it (HTML parsing would drop a
 * newline at the start of a pre) */
function render(text: string): HTMLDivElement {
  const div = document.createElement("div");
  act(() => createRoot(div).render(<RichTextOutput text={text} />));
  return div;
}

/** What each piece of the output shows: text as is, a block as "[tex]" */
function pieces(text: string): string[] {
  const div = render(text);
  return [...div.querySelectorAll(".text-segment, .inline-katex")].map((el) =>
    el.classList.contains("inline-katex") ? "[tex]" : el.textContent!,
  );
}

describe("print and tex output", () => {
  it("leaves no blank line around a block", () => {
    expect(pieces("before\n$$x^2$$\na = x^2\nn = 1 :\n$${{1}\\over{2}}$$\nafter")).toEqual([
      "before",
      "[tex]",
      "a = x^2\nn = 1 :",
      "[tex]",
      "after",
    ]);
  });

  it("puts nothing between blocks in a row", () => {
    expect(pieces("label:\n$$\\sqrt{x}$$\n$$\\arctan x$$\ntail")).toEqual(["label:", "[tex]", "[tex]", "tail"]);
  });

  it("keeps a blank line the user printed", () => {
    expect(pieces("$$x$$\n\nafter")).toEqual(["[tex]", "\nafter"]);
  });

  it("shows text without blocks as Maxima printed it", () => {
    const div = render("one\n\ntwo\n");
    expect(div.querySelector("pre")!.textContent).toBe("one\n\ntwo\n");
  });
});
