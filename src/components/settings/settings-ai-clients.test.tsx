// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn(), ask: vi.fn(), message: vi.fn() }));

import { SettingsModal } from "../SettingsModal";
import { answer, baseConfig, button, calls, click, clientsStatus, mount, unmount } from "./settings-test-kit";

const noop = () => {};

function row(container: HTMLElement, client: string): HTMLElement {
  const el = container.querySelector<HTMLElement>(`[data-client="${client}"]`);
  if (!el) throw new Error(`no row for ${client}`);
  return el;
}

afterEach(unmount);

describe("the AI assistant rows", () => {
  it("run no client's program when Settings opens", async () => {
    answer({ get_config: { config: baseConfig, warnings: [] }, mcp_clients_status: clientsStatus });
    const container = await mount(<SettingsModal onClose={noop} onSetVariablesOpen={noop} />);
    expect(calls()).toContain("mcp_clients_status");
    expect(calls()).not.toContain("claude_mcp_check");
    expect(calls().filter((c) => c.endsWith("_configure"))).toEqual([]);
    expect(row(container, "claude").textContent).toContain("Not checked");
    expect(row(container, "codex").textContent).toContain("Configured");
    expect(row(container, "gemini").textContent).toContain("gemini CLI not found");
  });

  it("check Claude Code only when asked", async () => {
    answer({
      get_config: { config: baseConfig, warnings: [] },
      mcp_clients_status: clientsStatus,
      claude_mcp_check: { state: "yes" },
    });
    const container = await mount(<SettingsModal onClose={noop} onSetVariablesOpen={noop} />);
    await click(button(row(container, "claude"), "Check"));
    expect(calls().filter((c) => c === "claude_mcp_check")).toHaveLength(1);
    expect(row(container, "claude").textContent).toContain("Configured");
  });

  it("check Claude Code once after Configure", async () => {
    answer({
      get_config: { config: baseConfig, warnings: [] },
      mcp_clients_status: clientsStatus,
      claude_mcp_configure: "ok",
      claude_mcp_check: { state: "yes" },
    });
    const container = await mount(<SettingsModal onClose={noop} onSetVariablesOpen={noop} />);
    await click(button(row(container, "claude"), "Configure"));
    const after = calls().slice(calls().indexOf("claude_mcp_configure"));
    expect(after).toEqual(["claude_mcp_configure", "claude_mcp_check"]);
  });

  it("disable Configure for a client that isn't installed", async () => {
    answer({ get_config: { config: baseConfig, warnings: [] }, mcp_clients_status: clientsStatus });
    const container = await mount(<SettingsModal onClose={noop} onSetVariablesOpen={noop} />);
    expect(button(row(container, "gemini"), "Configure")?.disabled).toBe(true);
  });

  it("name a client's file it can't read", async () => {
    answer({
      get_config: { config: baseConfig, warnings: [] },
      mcp_clients_status: {
        ...clientsStatus,
        codex: { installed: "/bin/codex", configured: { state: "error", file: "/home/u/.codex/config.toml", detail: "bad" } },
      },
    });
    const container = await mount(<SettingsModal onClose={noop} onSetVariablesOpen={noop} />);
    expect(row(container, "codex").textContent).toContain("Can't read /home/u/.codex/config.toml");
  });
});
