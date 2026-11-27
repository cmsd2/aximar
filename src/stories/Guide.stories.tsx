import type { Meta, StoryObj } from "@storybook/react-vite";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import guide from "../../docs/design-library.md?raw";

// The guide is docs/design-library.md itself, so there is one copy
const meta: Meta = { title: "Guide", tags: ["themes", "english-only"] };
export default meta;

const styles = `
.design-guide { max-width: 860px; font-family: var(--font-ui); font-size: var(--text-md); line-height: var(--leading-normal); color: var(--text-primary); }
.design-guide h1 { font-size: var(--text-2xl); margin: 0 0 var(--space-3); }
.design-guide h2 { font-size: var(--text-xl); margin: var(--space-6) 0 var(--space-2); }
.design-guide h3 { font-size: var(--text-lg); margin: var(--space-4) 0 var(--space-2); }
.design-guide p, .design-guide ul { margin: 0 0 var(--space-3); }
.design-guide ul { padding-left: var(--space-5); }
.design-guide code { font-family: var(--font-mono); font-size: var(--text-sm); background: var(--bg-secondary); padding: 0 var(--space-1); border-radius: var(--radius-sm); }
.design-guide a { color: var(--accent); }
.design-guide table { border-collapse: collapse; width: 100%; margin: 0 0 var(--space-3); }
.design-guide th, .design-guide td { text-align: left; vertical-align: top; padding: var(--space-1) var(--space-2); border-bottom: 0.5px solid var(--border-color); }
.design-guide th { font-weight: 500; color: var(--text-secondary); font-size: var(--text-sm); }
`;

/** A link to a story opens it in Storybook's own frame, not inside this story */
function Link({ href, children }: { href?: string; children?: React.ReactNode }) {
  const story = href?.startsWith("?path=");
  return (
    <a href={story ? `./${href}` : href} target={story ? "_top" : undefined}>
      {children}
    </a>
  );
}

/** docs/design-library.md: when to use each token and part, and the checks */
export const DesignLibrary: StoryObj = {
  render: () => (
    <div className="design-guide">
      <style>{styles}</style>
      <ReactMarkdown remarkPlugins={[remarkGfm]} components={{ a: Link }}>
        {guide}
      </ReactMarkdown>
    </div>
  ),
};
