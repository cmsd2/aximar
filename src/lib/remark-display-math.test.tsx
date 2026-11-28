// @vitest-environment happy-dom
import { describe, expect, it } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import ReactMarkdown from "react-markdown";
import remarkMath from "remark-math";
import rehypeKatex from "rehype-katex";
import { remarkDisplayMath } from "./remark-display-math";

function render(md: string): HTMLElement {
  const div = document.createElement("div");
  div.innerHTML = renderToStaticMarkup(
    <ReactMarkdown remarkPlugins={[remarkMath, remarkDisplayMath]} rehypePlugins={[rehypeKatex]}>
      {md}
    </ReactMarkdown>,
  );
  return div;
}

describe("double-dollar maths", () => {
  it("is display maths on a line of its own", () => {
    expect(render("Before\n\n$$x^2$$\n\nAfter").querySelectorAll(".katex-display")).toHaveLength(1);
  });

  it("is display maths inside a sentence too, as in Jupyter", () => {
    expect(render("so $$y = 1$$ holds").querySelectorAll(".katex-display")).toHaveLength(1);
  });

  it("leaves single-dollar maths inline", () => {
    const div = render("with $z$ inline");
    expect(div.querySelector(".katex")).not.toBeNull();
    expect(div.querySelector(".katex-display")).toBeNull();
  });

  it("leaves the multi-line form as it was", () => {
    expect(render("$$\nx^2\n$$").querySelectorAll(".katex-display")).toHaveLength(1);
  });
});
