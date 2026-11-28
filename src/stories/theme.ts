// The Storybook theme switch: Light, Paper, Dark or Night, through the
// attribute the token blocks in src/styles/tokens.css respond to.
export type StoryTheme = "light" | "paper" | "dark" | "night";

export function applyStoryTheme(theme: string | undefined, root: HTMLElement = document.documentElement): StoryTheme {
  const chosen: StoryTheme = theme === "dark" || theme === "paper" || theme === "night" ? theme : "light";
  root.dataset.theme = chosen;
  return chosen;
}
