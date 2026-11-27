// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), ask: vi.fn(), message: vi.fn() }));

import { SettingsModal } from "../SettingsModal";
import type { SectionId } from "./sections";
import { answer, baseConfig, clientsStatus, mount, unmount } from "./settings-test-kit";

const noop = () => {};

async function statusLine(section: SectionId, which: string, answers: Record<string, unknown>) {
  answer({
    get_config: { config: baseConfig, warnings: [] },
    mcp_clients_status: clientsStatus,
    ...answers,
  });
  const container = await mount(<SettingsModal onClose={noop} onSetVariablesOpen={noop} initialSection={section} />);
  return container.querySelector(`[data-status="${which}"]`);
}

afterEach(unmount);

describe("the Maxima status line", () => {
  function parts(box: Element | null) {
    return {
      main: box?.querySelector(".status-box-main")?.textContent,
      detail: box?.querySelector(".status-box-detail")?.textContent ?? null,
    };
  }

  it.each([
    [{ path: "/opt/maxima/bin/maxima", source: "setting", problem: null }, "Found Maxima 5.50.0 at /opt/maxima/bin/maxima", "From the Maxima path setting.", "ok"],
    [{ path: "/x/maxima", source: "environment", problem: null }, "Found Maxima 5.50.0 at /x/maxima", "From the AXIMAR_MAXIMA_PATH environment variable.", "ok"],
    [{ path: "/opt/homebrew/bin/maxima", source: "standard-location", problem: null }, "Found Maxima 5.50.0 at /opt/homebrew/bin/maxima", "In a standard install location.", "ok"],
    [{ path: "/usr/games/maxima", source: "path", problem: null }, "Found Maxima 5.50.0 at /usr/games/maxima", "On PATH.", "ok"],
    [{ path: "/nope/maxima", source: "setting", problem: "not-found" }, "/nope/maxima doesn't exist.", null, "err"],
    [{ path: "/etc/hosts", source: "setting", problem: "not-executable" }, "/etc/hosts isn't a program Aximar can run.", null, "err"],
    [{ path: null, source: null, problem: "not-found" }, "Aximar found no Maxima. Install it, or set the Maxima path above.", null, "err"],
  ])("says where Maxima comes from: %o", async (location, main, detail, tone) => {
    const box = await statusLine("maxima", "maxima", { maxima_status: location, maxima_version: "5.50.0" });
    expect(parts(box)).toEqual({ main, detail });
    expect(box?.classList.contains(tone)).toBe(true);
  });

  it("leaves the version out when Maxima doesn't give one", async () => {
    const box = await statusLine("maxima", "maxima", {
      maxima_status: { path: "/opt/homebrew/bin/maxima", source: "standard-location", problem: null },
      maxima_version: null,
    });
    expect(parts(box).main).toBe("Found Maxima at /opt/homebrew/bin/maxima");
  });

  it("doesn't run a program that isn't there", async () => {
    const mocked = answer({
      get_config: { config: baseConfig, warnings: [] },
      maxima_status: { path: "/nope/maxima", source: "setting", problem: "not-found" },
    });
    await mount(<SettingsModal onClose={noop} onSetVariablesOpen={noop} initialSection="maxima" />);
    expect(mocked.mock.calls.map((c) => c[0])).not.toContain("maxima_version");
  });

  it("isn't shown for Docker", async () => {
    answer({ get_config: { config: { ...baseConfig, backend: "docker" }, warnings: [] } });
    const container = await mount(<SettingsModal onClose={noop} onSetVariablesOpen={noop} initialSection="maxima" />);
    expect(container.querySelector('[data-status="maxima"]')).toBeNull();
  });
});

describe("the MCP server status line", () => {
  it.each([
    [{ state: "listening", address: "127.0.0.1:19542" }, "Listening on 127.0.0.1:19542.", "ok"],
    [{ state: "starting", address: "127.0.0.1:19542" }, "Starting...", "busy"],
    [
      { state: "failed", address: "127.0.0.1:19542", reason: "Address already in use (os error 48)" },
      "Couldn't listen on 127.0.0.1:19542.Address already in use (os error 48)",
      "err",
    ],
  ])("reports %o", async (state, text, tone) => {
    const line = await statusLine("ai-assistants", "mcp-server", { mcp_server_status: state });
    expect(line?.textContent).toBe(text);
    expect(line?.classList.contains(tone)).toBe(true);
  });
});
