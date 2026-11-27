import type { Meta, StoryObj } from "@storybook/react-vite";
import { CONTRAST_PAIRS } from "../../styles/contrast-pairs";
import { contrast } from "../../styles/tokens";
import { cell, colourTokens, headCell, Heading, Name, page, tokens } from "./foundation-kit";

const meta: Meta = { title: "Foundations/Colours", tags: ["themes", "english-only"] };
export default meta;

function Swatch({ value }: { value: string }) {
  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-2)" }}>
      <span
        style={{
          width: 28,
          height: 20,
          borderRadius: "var(--radius-sm)",
          border: "0.5px solid var(--border-color)",
          background: value,
        }}
      />
      <Name>{value}</Name>
    </span>
  );
}

/** Every colour token, with its light, dark and print values */
export const Tokens: StoryObj = {
  render: () => (
    <div style={page}>
      <Heading note="From src/styles/tokens.css. The swatch in the first column follows the theme switch.">
        Colours
      </Heading>
      <table style={{ borderCollapse: "collapse", width: "100%" }}>
        <thead>
          <tr>
            <th style={headCell}>Token</th>
            <th style={headCell}>Now</th>
            <th style={headCell}>Light</th>
            <th style={headCell}>Dark</th>
            <th style={headCell}>Print</th>
          </tr>
        </thead>
        <tbody>
          {colourTokens().map(([name, light]) => (
            <tr key={name}>
              <td style={cell}>
                <Name>--{name}</Name>
              </td>
              <td style={cell}>
                <span
                  style={{
                    display: "inline-block",
                    width: 28,
                    height: 20,
                    borderRadius: "var(--radius-sm)",
                    border: "0.5px solid var(--border-color)",
                    background: `var(--${name})`,
                  }}
                />
              </td>
              <td style={cell}>
                <Swatch value={light} />
              </td>
              <td style={cell}>{tokens.dark.get(name) && <Swatch value={tokens.dark.get(name)!} />}</td>
              <td style={cell}>{tokens.print.get(name) && <Swatch value={tokens.print.get(name)!} />}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  ),
};

function Ratio({ text, background, value }: { text: string; background: string; value: number }) {
  const pass = value >= 4.5;
  return (
    <span style={{ display: "inline-flex", alignItems: "center", gap: "var(--space-2)" }}>
      <span
        style={{
          padding: "var(--space-half) var(--space-2)",
          borderRadius: "var(--radius-sm)",
          background,
          color: text,
          border: "0.5px solid var(--border-color)",
          fontSize: "var(--text-md)",
        }}
      >
        Aa
      </span>
      <span style={{ color: pass ? "var(--success-text)" : "var(--error-color)", fontSize: "var(--text-md)" }}>
        {value.toFixed(2)}:1
      </span>
    </span>
  );
}

/** Each text colour on each background it sits on, with its contrast ratio */
export const Contrast: StoryObj = {
  render: () => (
    <div style={page}>
      <Heading note="From src/styles/contrast-pairs.ts. Each pair needs 4.5:1 in both themes; src/styles/tokens.test.ts checks it.">
        Contrast
      </Heading>
      <table style={{ borderCollapse: "collapse", width: "100%" }}>
        <thead>
          <tr>
            <th style={headCell}>Text</th>
            <th style={headCell}>On</th>
            <th style={headCell}>Light</th>
            <th style={headCell}>Dark</th>
            <th style={headCell}>Exempt</th>
          </tr>
        </thead>
        <tbody>
          {CONTRAST_PAIRS.map(({ text, background, exempt }) => {
            const light = [tokens.light.get(text)!, tokens.light.get(background)!];
            const dark = [tokens.dark.get(text)!, tokens.dark.get(background)!];
            return (
              <tr key={`${text} ${background}`}>
                <td style={cell}>
                  <Name>--{text}</Name>
                </td>
                <td style={cell}>
                  <Name>--{background}</Name>
                </td>
                <td style={cell}>
                  <Ratio text={light[0]} background={light[1]} value={contrast(light[0], light[1])} />
                </td>
                <td style={cell}>
                  <Ratio text={dark[0]} background={dark[1]} value={contrast(dark[0], dark[1])} />
                </td>
                <td style={{ ...cell, color: "var(--text-secondary)", fontSize: "var(--text-sm)" }}>{exempt ?? ""}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  ),
};
