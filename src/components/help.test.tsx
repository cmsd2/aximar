// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot, type Root } from "react-dom/client";
import { I18nProvider } from "@lingui/react";

(globalThis as { IS_REACT_ACT_ENVIRONMENT?: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

const openUrl = vi.fn(async (_url: string) => {});
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/window", () => ({ getCurrentWindow: () => ({ close: vi.fn() }) }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: (url: string) => openUrl(url) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ message: vi.fn(), ask: vi.fn(), open: vi.fn(), save: vi.fn() }));

import { i18n } from "../lib/i18n";
import { COMMANDS, command, type CommandContext } from "../commands/registry";
import { formatShortcut, setPlatformForTests } from "../commands/shortcut";
import { useUiStore } from "../store/uiStore";
import { ShortcutsDialog } from "./ShortcutsDialog";

const ctx: CommandContext = { executeCell: vi.fn(), runAll: vi.fn(), restartSession: vi.fn() };

describe("Help", () => {
  let root: Root | null = null;
  afterEach(() => {
    if (root) act(() => root!.unmount());
    root = null;
    setPlatformForTests(null);
  });

  it("opens the release notes and a new issue on GitHub", async () => {
    await command("help.whats_new")!.run(ctx);
    await command("help.report_issue")!.run(ctx);
    expect(openUrl.mock.calls.map((c) => c[0])).toEqual([
      "https://github.com/cmsd2/aximar/releases",
      "https://github.com/cmsd2/aximar/issues/new",
    ]);
  });

  it("opens the docs panel for Aximar Help", () => {
    useUiStore.setState({ docsOpen: false });
    void command("help.docs")!.run(ctx);
    expect(useUiStore.getState().docsOpen).toBe(true);
  });

  it("lists every menu shortcut once in the Keyboard Shortcuts dialog", () => {
    setPlatformForTests("windows");
    const div = document.createElement("div");
    document.body.appendChild(div);
    root = createRoot(div);
    act(() => root!.render(<I18nProvider i18n={i18n}><ShortcutsDialog onClose={() => {}} /></I18nProvider>));
    const rows = [...document.querySelectorAll("[data-command]")];
    for (const c of COMMANDS.filter((c) => c.shortcut)) {
      const matching = rows.filter((r) => r.getAttribute("data-command") === c.id);
      expect(matching, c.id).toHaveLength(1);
      expect(matching[0].textContent).toContain(formatShortcut(c.shortcut!, "windows"));
    }
    expect(document.body.textContent).toContain("Shift+Enter");
  });
});
