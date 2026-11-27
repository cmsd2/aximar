// @vitest-environment happy-dom
import { afterEach, describe, expect, it, vi } from "vitest";
import { act } from "react";
import { createRoot } from "react-dom/client";
import { renderToStaticMarkup } from "react-dom/server";
import { I18nProvider } from "@lingui/react";
import { compileMessage } from "@lingui/message-utils/compileMessage";
import { generateMessageId } from "@lingui/message-utils/generateMessageId";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("./PlotlyChart", () => ({ PlotlyChart: () => null }));

import { i18n } from "../lib/i18n";
import { TextWithNotes } from "./TextWithNotes";
import type { Note } from "../types/maxima";

const text = [
  "hello",
  "rat: replaced 0.25 by 1/4 = 0.25",
  "rat: replaced 0.5 by 1/2 = 0.5",
  "bye",
].join("\n");

const template = "Maxima replaced decimals with exact fractions: {replacements}.";
const ratNote: Note = {
  id: "note.rat_replaced",
  message: {
    id: "note.rat_replaced.text",
    template,
    params: { replacements: "0.25 → 1/4, 0.5 → 1/2" },
    text: "Maxima replaced decimals with exact fractions: 0.25 → 1/4, 0.5 → 1/2.",
  },
  start: 1,
  end: 3,
};

function element(notes: Note[]) {
  return (
    <I18nProvider i18n={i18n}>
      <TextWithNotes text={text} notes={notes} />
    </I18nProvider>
  );
}

afterEach(() => {
  i18n.activate("en");
  document.body.innerHTML = "";
});

describe("notes in a cell's output", () => {
  it("replace Maxima's lines with one note line, keeping the text around them", () => {
    const html = renderToStaticMarkup(element([ratNote]));
    expect(html).toContain("hello");
    expect(html).toContain("bye");
    expect(html).toContain("0.25 → 1/4, 0.5 → 1/2");
    expect(html).not.toContain("rat: replaced");
    expect(html.indexOf("hello")).toBeLessThan(html.indexOf("Maxima replaced"));
    expect(html.indexOf("Maxima replaced")).toBeLessThan(html.indexOf("bye"));
  });

  it("show in the user's language", () => {
    i18n.load("es", {
      [generateMessageId(template, "note.rat_replaced.text")]: compileMessage(
        "Maxima sustituyó decimales por fracciones exactas: {replacements}.",
      ),
    });
    i18n.activate("es");
    const html = renderToStaticMarkup(element([ratNote]));
    expect(html).toContain("Maxima sustituyó decimales por fracciones exactas: 0.25 → 1/4, 0.5 → 1/2.");
  });

  it("show Maxima's lines in place on request", async () => {
    const container = document.createElement("div");
    document.body.appendChild(container);
    const root = createRoot(container);
    await act(async () => root.render(element([ratNote])));
    expect(container.textContent).not.toContain("rat: replaced");

    const toggle = container.querySelector<HTMLButtonElement>(".output-note-toggle")!;
    expect(toggle.textContent).toBe("Show Maxima's text");
    await act(async () => toggle.click());
    const maxima = container.querySelector(".output-note-maxima")!;
    expect(maxima.textContent).toBe(
      "rat: replaced 0.25 by 1/4 = 0.25\nrat: replaced 0.5 by 1/2 = 0.5",
    );
    expect(toggle.textContent).toBe("Hide Maxima's text");
    root.unmount();
  });

  it("leave text without notes as Maxima printed it", () => {
    const html = renderToStaticMarkup(element([]));
    expect(html).toContain("rat: replaced 0.5 by 1/2 = 0.5");
  });
});
