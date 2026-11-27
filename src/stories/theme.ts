// The Storybook theme switch: Light or Dark, through the attribute the
// token blocks in src/styles/tokens.css respond to.
export type StoryTheme = "light" | "dark";

export function applyStoryTheme(theme: string | undefined, root: HTMLElement = document.documentElement): StoryTheme {
  const chosen: StoryTheme = theme === "dark" ? "dark" : "light";
  root.dataset.theme = chosen;
  return chosen;
}
