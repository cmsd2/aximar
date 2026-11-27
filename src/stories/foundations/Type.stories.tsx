import type { Meta, StoryObj } from "@storybook/react-vite";
import { cell, headCell, Heading, Name, page, tokensNamed } from "./foundation-kit";

const meta: Meta = { title: "Foundations/Type", tags: ["themes", "english-only"] };
export default meta;

const SAMPLE = "integrate(sin(x)^2, x, 0, π) = π/2";

/** The type scale in the UI font and the monospace font, and the line heights */
export const Scale: StoryObj = {
  render: () => (
    <div style={page}>
      <Heading note="--font-ui is the system's UI font; --font-mono is for code, output and paths.">Type</Heading>
      <table style={{ borderCollapse: "collapse", width: "100%" }}>
        <thead>
          <tr>
            <th style={headCell}>Token</th>
            <th style={headCell}>Size</th>
            <th style={headCell}>--font-ui</th>
            <th style={headCell}>--font-mono</th>
          </tr>
        </thead>
        <tbody>
          {tokensNamed("text-").map(([name, value]) => (
            <tr key={name}>
              <td style={cell}>
                <Name>--{name}</Name>
              </td>
              <td style={cell}>
                <Name>{value}</Name>
              </td>
              <td style={{ ...cell, fontSize: `var(--${name})`, fontFamily: "var(--font-ui)" }}>Evaluation timeout</td>
              <td style={{ ...cell, fontSize: `var(--${name})`, fontFamily: "var(--font-mono)" }}>{SAMPLE}</td>
            </tr>
          ))}
        </tbody>
      </table>
      <h2 style={{ fontSize: "var(--text-xl)", fontWeight: 600, margin: "var(--space-6) 0 var(--space-2)" }}>
        Line heights
      </h2>
      {tokensNamed("leading-").map(([name, value]) => (
        <div key={name} style={{ display: "flex", gap: "var(--space-4)", marginBottom: "var(--space-4)" }}>
          <div style={{ width: 200 }}>
            <Name>--{name}</Name> <Name>{value}</Name>
          </div>
          <p
            style={{
              margin: 0,
              maxWidth: 420,
              fontSize: "var(--text-md)",
              lineHeight: `var(--${name})`,
              background: "var(--bg-secondary)",
              padding: "var(--space-2)",
              borderRadius: "var(--radius-sm)",
            }}
          >
            Aximar couldn't find Maxima on your system. Install it, or give its path in the Maxima section of Settings.
          </p>
        </div>
      ))}
      <h2 style={{ fontSize: "var(--text-xl)", fontWeight: 600, margin: "var(--space-6) 0 var(--space-2)" }}>Fonts</h2>
      {tokensNamed("font-").map(([name, value]) => (
        <p key={name} style={{ margin: "0 0 var(--space-2)", fontSize: "var(--text-md)" }}>
          <Name>--{name}</Name>: <span style={{ fontFamily: `var(--${name})` }}>{value}</span>
        </p>
      ))}
    </div>
  ),
};
