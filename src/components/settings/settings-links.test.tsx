// @vitest-environment happy-dom
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { ask } from "@tauri-apps/plugin-dialog";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), ask: vi.fn(), message: vi.fn() }));

import { useSettingsStore } from "../../store/settingsStore";
import { showMaximaNotFoundDialog, resetMaximaDialog } from "../../hooks/useMaxima";
import { LogLinkButton } from "../LogPanel";
import { SettingsModal } from "../SettingsModal";
import { answer, baseConfig, button, click, clientsStatus, mount, settle, unmount } from "./settings-test-kit";
import { act } from "react";

const noop = () => {};

beforeEach(() => {
  useSettingsStore.setState({ open: false, section: null, request: 0 });
  resetMaximaDialog();
});
afterEach(unmount);

describe("opening Settings at a section", () => {
  it("opens at the remembered section when none is named", () => {
    useSettingsStore.getState().openSettings();
    expect(useSettingsStore.getState()).toMatchObject({ open: true, section: null });
  });

  it("makes a named section the remembered one", async () => {
    const saved: unknown[] = [];
    answer({
      get_config: { config: baseConfig, warnings: [] },
      mcp_clients_status: clientsStatus,
      set_config: ({ updates }: Record<string, unknown>) => saved.push(updates),
    });
    const container = await mount(<SettingsModal onClose={noop} onSetVariablesOpen={noop} initialSection="printing" />);
    expect(container.querySelector(".settings-page")?.getAttribute("data-section")).toBe("printing");
    expect(saved).toContainEqual({ settings_section: "printing" });
  });

  it("moves an open window to the section asked for", async () => {
    answer({ get_config: { config: baseConfig, warnings: [] }, mcp_clients_status: clientsStatus });
    function Open() {
      const { section, request } = useSettingsStore();
      return (
        <SettingsModal onClose={noop} onSetVariablesOpen={noop} initialSection={section ?? undefined} request={request} />
      );
    }
    const container = await mount(<Open />);
    expect(container.querySelector(".settings-page")?.getAttribute("data-section")).toBe("general");
    await act(async () => useSettingsStore.getState().openSettings("maxima"));
    await settle();
    expect(container.querySelector(".settings-page")?.getAttribute("data-section")).toBe("maxima");
  });
});

describe("the Maxima Not Found dialog", () => {
  it("opens the Maxima section on Open Settings", async () => {
    vi.mocked(ask).mockResolvedValueOnce(true);
    await showMaximaNotFoundDialog("No such file or directory (os error 2)");
    expect(vi.mocked(ask).mock.calls[0][1]).toMatchObject({ okLabel: "Open Settings", cancelLabel: "Close" });
    expect(useSettingsStore.getState()).toMatchObject({ open: true, section: "maxima" });
  });

  it("leaves Settings closed on Close", async () => {
    vi.mocked(ask).mockResolvedValueOnce(false);
    await showMaximaNotFoundDialog("No such file or directory (os error 2)");
    expect(useSettingsStore.getState().open).toBe(false);
  });
});

describe("a log entry's link", () => {
  it("opens its Settings section", async () => {
    const container = await mount(<LogLinkButton section="ai-assistants" />);
    await click(button(container, "Open AI assistants settings"));
    expect(useSettingsStore.getState()).toMatchObject({ open: true, section: "ai-assistants" });
  });
});
