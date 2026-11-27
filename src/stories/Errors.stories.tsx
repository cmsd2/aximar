import type { Meta, StoryObj } from "@storybook/react-vite";
import { EnhancedErrorOutput } from "../components/EnhancedErrorOutput";
import { fixtures } from "./fixtures";

const meta: Meta = { title: "Errors" };
export default meta;

/** Every error explanation and question the core gives, as a failed cell shows it */
export const AllExplanations: StoryObj = {
  render: () => (
    <div>
      {fixtures.explanations.map(({ error, error_info }, i) => (
        <section key={i} style={{ marginBottom: 24 }}>
          <code style={{ fontSize: 11, color: "#888" }}>{error_info.id}</code>
          <EnhancedErrorOutput error={error} errorInfo={error_info} cellId="c1" />
        </section>
      ))}
    </div>
  ),
};

const divisionByZero = fixtures.explanations[0];

/** A Maxima error with Maxima's own message in the user's language */
export const TranslatedMaximaError: StoryObj = {
  render: () => (
    <EnhancedErrorOutput
      error={divisionByZero.error}
      errorInfo={divisionByZero.error_info}
      errorTranslated="expt: no definido: desde 0 a un exponente negativo."
      cellId="c1"
    />
  ),
};

/** A Maxima error Aximar has no explanation for, translated by Maxima */
export const MaximaErrorWithoutExplanation: StoryObj = {
  render: () => (
    <EnhancedErrorOutput
      error="diff: wrong number of arguments."
      errorInfo={null}
      errorTranslated="diff: número de argumentos equivocados."
      cellId="c1"
    />
  ),
};
