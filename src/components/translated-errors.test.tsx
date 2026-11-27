import { afterEach, describe, expect, it, vi } from "vitest";
import { renderToStaticMarkup } from "react-dom/server";
import { I18nProvider } from "@lingui/react";
import { compileMessage } from "@lingui/message-utils/compileMessage";
import { generateMessageId } from "@lingui/message-utils/generateMessageId";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { i18n } from "../lib/i18n";
import { EnhancedErrorOutput } from "./EnhancedErrorOutput";
import type { ErrorInfo } from "../types/maxima";

/** The core's explanation of 1/0, as JSON gives it */
const divisionByZero: ErrorInfo = {
  id: "error.division_by_zero",
  params: {},
  messages: {
    title: { id: "error.division_by_zero.title", template: "Division by Zero" },
    explanation: {
      id: "error.division_by_zero.explanation",
      template:
        "The expression involves division by zero or raising zero to a negative power, which is undefined.",
    },
  },
  title: "Division by Zero",
  explanation:
    "The expression involves division by zero or raising zero to a negative power, which is undefined.",
  suggestion: null,
  example: null,
  did_you_mean: [],
  correct_signatures: [],
};
const english = "expt: undefined: 0 to a negative exponent.";
const spanish = "expt: no definido: desde 0 a un exponente negativo.";

function activateSpanish() {
  const entries: [string, string, string][] = [
    ["error.division_by_zero.title", "Division by Zero", "División por cero"],
    ["", "Show original", "Mostrar original"],
  ];
  i18n.load(
    "es",
    Object.fromEntries(
      entries.map(([context, en, es]) => [generateMessageId(en, context || undefined), compileMessage(es)]),
    ),
  );
  i18n.activate("es");
}

function render(errorTranslated: string | null) {
  return renderToStaticMarkup(
    <I18nProvider i18n={i18n}>
      <EnhancedErrorOutput
        error={english}
        errorInfo={divisionByZero}
        errorTranslated={errorTranslated}
        cellId="c1"
      />
    </I18nProvider>,
  );
}

afterEach(() => {
  i18n.activate("en");
});

describe("Maxima's errors in the user's language", () => {
  it("shows the explanation, then Maxima's translation, with the English one click away", () => {
    activateSpanish();
    const html = render(spanish);
    expect(html).toContain("División por cero");
    expect(html).toContain(spanish);
    expect(html.indexOf("raising zero")).toBeLessThan(html.indexOf(spanish));
    expect(html).toContain("Mostrar original");
    // The English is behind the button, not on screen
    expect(html).not.toContain(english);
  });

  it("offers Maxima's text as it is when Maxima has no translation", () => {
    activateSpanish();
    const html = render(null);
    expect(html).toContain("División por cero");
    expect(html).toMatch(/Show Maxima(&#x27;|')s error/);
    expect(html).not.toContain("Mostrar original");
  });

  it("shows no translation in English", () => {
    const html = render(null);
    expect(html).toContain("Division by Zero");
    expect(html).not.toContain(spanish);
    expect(html).toMatch(/Show Maxima(&#x27;|')s error/);
  });
});
