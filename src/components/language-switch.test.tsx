import { afterEach, describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { I18nProvider } from "@lingui/react";
import { compileMessage } from "@lingui/message-utils/compileMessage";
import { generateMessageId } from "@lingui/message-utils/generateMessageId";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { i18n } from "../lib/i18n";
import { errorText } from "../lib/messages";
import { EnhancedErrorOutput } from "./EnhancedErrorOutput";
import type { ErrorInfo } from "../types/maxima";

/** The core's explanation of `f(1, 2)` with f defined as f(x), as JSON gives it */
const argCount: ErrorInfo = {
  id: "error.arg_count",
  params: { func: "f", definition: "f(x)", found: "[1,2]" },
  messages: {
    title: { id: "error.arg_count.title", template: "Wrong Argument Count" },
    explanation: {
      id: "error.arg_count.explanation",
      template: "{func} is defined as {definition}, but was called with {found}.",
    },
    suggestion: {
      id: "error.arg_count.suggestion",
      template: "Call {func} with the arguments {definition} names, or change its definition.",
    },
  },
  title: "Wrong Argument Count",
  explanation: "f is defined as f(x), but was called with [1,2].",
  suggestion: "Call f with the arguments f(x) names, or change its definition.",
  example: null,
  did_you_mean: [],
  correct_signatures: [],
};

/** A Spanish catalog with these translations, by context and English */
function loadSpanish(entries: [string | undefined, string, string][]) {
  const messages = Object.fromEntries(
    entries.map(([context, english, spanish]) => [
      generateMessageId(english, context),
      compileMessage(spanish),
    ]),
  );
  i18n.load("es", messages);
}

function render() {
  return renderToStaticMarkup(
    <I18nProvider i18n={i18n}>
      <EnhancedErrorOutput error="Too many arguments" errorInfo={argCount} cellId="c1" />
    </I18nProvider>,
  );
}

afterEach(() => {
  i18n.activate("en");
});

describe("switching language", () => {
  it("re-renders an existing explanation in the new language, with its parameters", () => {
    expect(render()).toContain("f is defined as f(x), but was called with [1,2].");

    loadSpanish([
      ["error.arg_count.title", "Wrong Argument Count", "Número de argumentos incorrecto"],
      [
        "error.arg_count.explanation",
        "{func} is defined as {definition}, but was called with {found}.",
        "{func} se definió como {definition}, pero se llamó con {found}.",
      ],
    ]);
    i18n.activate("es");

    const html = render();
    expect(html).toContain("Número de argumentos incorrecto");
    expect(html).toContain("f se definió como f(x), pero se llamó con [1,2].");
  });

  it("shows English where a translation is missing", () => {
    loadSpanish([["error.arg_count.title", "Wrong Argument Count", "Número de argumentos incorrecto"]]);
    i18n.activate("es");

    const html = render();
    expect(html).toContain("Número de argumentos incorrecto");
    // Neither the suggestion nor the window's button is in this catalog
    expect(html).toContain("Call f with the arguments f(x) names, or change its definition.");
    expect(html).toContain("Show raw error");
    expect(html).not.toContain("{func}");
  });

  it("shows the English text of an explanation from before IDs", () => {
    const old: ErrorInfo = { ...argCount, id: undefined, params: undefined, messages: undefined };
    loadSpanish([["error.arg_count.title", "Wrong Argument Count", "Número de argumentos incorrecto"]]);
    i18n.activate("es");
    expect(errorText(i18n, old, "title")).toBe("Wrong Argument Count");
  });
});
