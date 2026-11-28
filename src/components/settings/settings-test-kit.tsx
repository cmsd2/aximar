// Mounting the Settings window in tests, with Tauri's commands answered by
// a table.  Call `vi.mock("@tauri-apps/api/core", ...)` and
// `vi.mock("@tauri-apps/plugin-dialog", ...)` in the test file first.
import { act, type ReactElement } from "react";
import { createRoot, type Root } from "react-dom/client";
import { I18nProvider } from "@lingui/react";
import { invoke } from "@tauri-apps/api/core";
import { vi } from "vitest";
import { i18n } from "../../lib/i18n";

// React checks updates are wrapped in act()
(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

export const baseConfig = {
  theme: "light", maxima_path: null, font_size: 14, print_font_size: 11, eval_timeout: 30,
  variables_open: false, cell_style: "bracket", autocomplete_mode: "active-hint", show_suggestions: true, system_title_bar: false,
  markdown_font: "sans-serif", markdown_indent: "aligned", print_margin_top: 15,
  print_margin_bottom: 15, print_margin_left: 15, print_margin_right: 15, backend: "local",
  docker_image: "aximar/maxima", wsl_distro: "", container_engine: "docker",
  mcp_enabled: true, mcp_listen_address: "127.0.0.1:19542", mcp_token: "0123456789abcdef",
  language: "en", languages: ["en", "es"], settings_section: "general",
};

export const clientsStatus = {
  claude: { installed: "/usr/local/bin/claude", configured: { state: "unchecked" } },
  codex: { installed: "/usr/local/bin/codex", configured: { state: "yes" } },
  gemini: { installed: null, configured: { state: "no" } },
};

export type Answers = Record<string, unknown | ((args: Record<string, unknown>) => unknown)>;

/** Answer each command from `answers`; unknown commands give null */
export function answer(answers: Answers) {
  const mocked = vi.mocked(invoke);
  mocked.mockReset();
  mocked.mockImplementation(async (cmd: string, args?: unknown) => {
    const a = answers[cmd];
    return typeof a === "function" ? a((args ?? {}) as Record<string, unknown>) : (a ?? null);
  });
  return mocked;
}

/** The commands called, in order */
export function calls(): string[] {
  return vi.mocked(invoke).mock.calls.map((c) => c[0] as string);
}

let root: Root | null = null;

export async function mount(element: ReactElement): Promise<HTMLElement> {
  const container = document.createElement("div");
  document.body.appendChild(container);
  root = createRoot(container);
  await act(async () => {
    root!.render(<I18nProvider i18n={i18n}>{element}</I18nProvider>);
  });
  await settle();
  return container;
}

/** Let pending promises and effects run */
export async function settle() {
  for (let i = 0; i < 5; i++) {
    await act(async () => {
      await new Promise((r) => setTimeout(r, 0));
    });
  }
}

export function unmount() {
  act(() => root?.unmount());
  root = null;
  document.body.innerHTML = "";
}

export async function click(el: Element | null) {
  if (!el) throw new Error("nothing to click");
  await act(async () => {
    (el as HTMLElement).click();
  });
  await settle();
}

export function button(container: HTMLElement, text: string): HTMLButtonElement | null {
  return (
    [...container.querySelectorAll("button")].find((b) => b.textContent?.trim() === text) ?? null
  );
}
