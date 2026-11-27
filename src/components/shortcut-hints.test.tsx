// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { I18nProvider } from "@lingui/react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ close: vi.fn() }) }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ message: vi.fn(), ask: vi.fn(), open: vi.fn(), save: vi.fn() }));

import { i18n } from "../lib/i18n";
import { setPlatformForTests } from "../commands/shortcut";
import { ShortcutHints } from "./ShortcutHints";

function render(): string {
  return renderToStaticMarkup(<I18nProvider i18n={i18n}><ShortcutHints held /></I18nProvider>);
}

describe("the shortcut hints", () => {
  afterEach(() => setPlatformForTests(null));

  it("use Ctrl on Windows, never Cmd or ⌘", () => {
    setPlatformForTests("windows");
    const html = render();
    expect(html).toContain("Ctrl+Z");
    expect(html).not.toMatch(/Cmd|⌘/);
  });

  it("use ⌘ on macOS", () => {
    setPlatformForTests("macos");
    expect(render()).toContain("⌘Z");
  });

  it("come from the command list, so Delete Cell shows its new keys", () => {
    setPlatformForTests("windows");
    expect(render()).toContain("Ctrl+Shift+Backspace");
  });
});
