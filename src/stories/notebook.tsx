// The seeded notebook as the window shows it: each cell in its own view
// (code, Markdown or raw), with the insert bars and selection brackets
import { Notebook } from "../components/Notebook";

const noop = () => {};

export function StoryNotebook() {
  return <Notebook onViewDocs={noop} />;
}
