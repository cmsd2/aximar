// Maths written between double dollars is display maths, as in Jupyter and
// LaTeX, wherever it sits.  remark-math only treats $$ as display maths
// when the dollars are on lines of their own; written on one line,
// `$$x^2$$` comes through as inline maths.  This marks those for
// rehype-katex to render in display mode.
import type { Plugin } from "unified";
import type { Root, Nodes } from "mdast";

export const remarkDisplayMath: Plugin<[], Root> = () => (tree, file) => {
  const source = String(file.value);
  const visit = (node: Nodes) => {
    if (node.type === "inlineMath") {
      const at = node.position?.start.offset;
      if (at !== undefined && source.startsWith("$$", at)) {
        node.data = {
          ...node.data,
          hProperties: { ...node.data?.hProperties, className: ["language-math", "math-display"] },
        };
      }
    }
    if ("children" in node) node.children.forEach((child) => visit(child as Nodes));
  };
  visit(tree);
};
