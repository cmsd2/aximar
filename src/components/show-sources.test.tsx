// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { I18nProvider } from "@lingui/react";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { i18n } from "../lib/i18n";
import { decodeIds, encodeId, installSourceTags, setShowSources } from "../lib/show-sources";
import { loadSources } from "../lib/translation-preview";
import { EnhancedErrorOutput } from "./EnhancedErrorOutput";
import { SourcesOverlay } from "./SourcesOverlay";

installSourceTags(i18n);

afterEach(() => {
  setShowSources(i18n, false);
  document.body.innerHTML = "";
});

describe("show sources", () => {
  it("encodes IDs in invisible characters and reads them back", () => {
    const text = `Show raw error${encodeId("ip5Gnc")} and more${encodeId("a+b/C_")}`;
    expect(decodeIds(text)).toEqual(["ip5Gnc", "a+b/C_"]);
    expect(text.replace(/[​‌⁣⁤]/g, "")).toBe("Show raw error and more");
  });

  it("tags strings only while the mode is on", () => {
    const show = () => i18n._({ id: "LkQRL8", message: "Show original" });
    expect(show()).toBe("Show original");
    setShowSources(i18n, true);
    expect(decodeIds(show())).toEqual(["LkQRL8"]);
    expect(i18n.locale).toBe("en-x-src");
    setShowSources(i18n, false);
    expect(i18n.locale).toBe("en");
  });

  it("shows a string's English, note and file when pointing at it with Alt", async () => {
    const sources = await loadSources();
    setShowSources(i18n, true);
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);
    await act(async () =>
      root.render(
        <I18nProvider i18n={i18n}>
          <EnhancedErrorOutput error="diff: wrong number of arguments." errorInfo={null} errorTranslated="diff: número de argumentos equivocados." cellId="c1" />
          <SourcesOverlay sources={sources} />
        </I18nProvider>,
      ),
    );
    const button = container.querySelector<HTMLButtonElement>(".enhanced-error-toggle")!;
    await act(async () => {
      button.dispatchEvent(new MouseEvent("mouseover", { bubbles: true, altKey: true, clientX: 10, clientY: 10 }));
    });
    const panel = container.querySelector(".sources-overlay");
    expect(panel, container.innerHTML).not.toBeNull();
    expect(panel!.textContent).toContain("Show original");
    expect(panel!.textContent).toContain("Button: shows Maxima's English error message below its translation");
    expect(panel!.textContent).toContain("src/components/EnhancedErrorOutput.tsx");

    // Without Alt, nothing
    await act(async () => {
      button.dispatchEvent(new MouseEvent("mouseover", { bubbles: true, clientX: 10, clientY: 10 }));
    });
    expect(container.querySelector(".sources-overlay")).toBeNull();
    root.unmount();
  });
});
