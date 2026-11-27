import type { Meta, StoryObj } from "@storybook/react-vite";
import { DocsPanel } from "../components/DocsPanel";
import { setStoryAnswers } from "../../.storybook/tauri-mock";
import type { DocsLanguages, MaximaFunction } from "../types/catalog";

const meta: Meta = { title: "Docs panel" };
export default meta;

const BODY = `Attempts to symbolically compute the integral of *expr* with respect to *x*.
\`integrate(expr, x)\` is an indefinite integral, while \`integrate(expr, x, a, b)\` is a definite integral.`;

const spanish = (full: boolean, pack: string | null): DocsLanguages => ({
  choice: null,
  effective: "es",
  languages: [
    { tag: "en", full: true, pack: null },
    { tag: "es", full, pack },
  ],
});

const integrate = (languages: MaximaFunction["languages"], description = BODY): MaximaFunction => ({
  name: "integrate",
  signatures: ["integrate(expr, x)", "integrate(expr, x, a, b)"],
  description,
  category: "Calculus",
  examples: [],
  see_also: ["diff", "risch"],
  languages,
});

function story(fn: MaximaFunction, info: DocsLanguages, docs: string | null = fn.description): StoryObj {
  return {
    loaders: [
      async () => {
        setStoryAnswers({ docs_languages: info, get_function: fn, get_function_docs: docs });
        return {};
      },
    ],
    render: () => (
      <div style={{ height: 640, display: "flex" }}>
        <DocsPanel open functionName="integrate" requestId={1} onClose={() => {}} />
      </div>
    ),
  };
}

/** Spanish help with only Aximar's Spanish summaries: a Spanish summary above English documentation */
export const SummaryAboveEnglish = story(
  integrate({ summary: "es", body: "en", stale: false, summary_text: "Calcula simbólicamente la integral de expr con respecto a x." }),
  spanish(false, "maxima-core-docs-es"),
);

/** A function with nothing in the help's language */
export const NotInTheHelpLanguage = story(
  integrate({ summary: "en", body: "en", stale: false, fallback_from: "es" }),
  spanish(true, "maxima-core-docs-es"),
);

/** A translation whose signatures differ from the English */
export const OldTranslation = story(
  integrate(
    {
      summary: "es",
      body: "es",
      stale: true,
      english_signatures: ["integrate(expr, x)", "integrate(expr, x, a, b)"],
    },
    "Calcula simbólicamente la integral de *expr* con respecto a *x*.",
  ),
  spanish(true, "maxima-core-docs-es"),
);

/** Spanish help with no package of full Spanish documentation in the index yet */
export const NoPackYet = story(integrate({ summary: "es", body: "en", stale: false, summary_text: "Calcula simbólicamente la integral." }), spanish(false, null));
