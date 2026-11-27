import type { Meta, StoryObj } from "@storybook/react-vite";
import { Heading, Name, page, tokensNamed } from "./foundation-kit";

const meta: Meta = { title: "Foundations/Space, radii and shadows", tags: ["themes", "english-only"] };
export default meta;

const row = { display: "flex", alignItems: "center", gap: "var(--space-4)", marginBottom: "var(--space-3)" } as const;
const label = { width: 200, flexShrink: 0 } as const;

/** The spacing scale, on a 4px grid */
export const Spacing: StoryObj = {
  render: () => (
    <div style={page}>
      <Heading note="Gaps, paddings and margins use these steps.">Spacing</Heading>
      {tokensNamed("space-").map(([name, value]) => (
        <div key={name} style={row}>
          <div style={label}>
            <Name>--{name}</Name> <Name>{value}</Name>
          </div>
          <div style={{ width: `var(--${name})`, height: 16, background: "var(--accent)", borderRadius: 2 }} />
        </div>
      ))}
    </div>
  ),
};

/** The corner radii and the shadows */
export const RadiiAndShadows: StoryObj = {
  render: () => (
    <div style={page}>
      <Heading note="Small for controls and badges, medium for buttons, fields and boxes, large for dialogs.">
        Radii and shadows
      </Heading>
      <div style={{ display: "flex", gap: "var(--space-6)", marginBottom: "var(--space-8)" }}>
        {tokensNamed("radius-").map(([name, value]) => (
          <figure key={name} style={{ margin: 0 }}>
            <div
              style={{
                width: 96,
                height: 64,
                border: "1px solid var(--border-color)",
                background: "var(--bg-secondary)",
                borderRadius: `var(--${name})`,
              }}
            />
            <figcaption style={{ marginTop: "var(--space-2)" }}>
              <Name>--{name}</Name> <Name>{value}</Name>
            </figcaption>
          </figure>
        ))}
      </div>
      <div style={{ display: "flex", gap: "var(--space-8)", padding: "var(--space-4)" }}>
        {tokensNamed("shadow-").map(([name, value]) => (
          <figure key={name} style={{ margin: 0 }}>
            <div
              style={{
                width: 200,
                height: 96,
                background: "var(--bg-primary)",
                border: "0.5px solid var(--border-color)",
                borderRadius: "var(--radius-lg)",
                boxShadow: `var(--${name})`,
              }}
            />
            <figcaption style={{ marginTop: "var(--space-3)" }}>
              <Name>--{name}</Name>
              <br />
              <Name>{value}</Name>
            </figcaption>
          </figure>
        ))}
      </div>
    </div>
  ),
};
