import type { Meta, StoryObj } from "@storybook/react-vite";
import type { ComponentType } from "react";
import * as icons from "../../components/ui/icons";
import iconsSource from "../../components/ui/icons.ts?raw";
import { cell, headCell, Heading, Name, page } from "./foundation-kit";

const meta: Meta = { title: "Foundations/Icons", tags: ["themes", "english-only"] };
export default meta;

/** Each icon's use, from the doc comment above its export in icons.ts */
function uses(): [string, string][] {
  return [...iconsSource.matchAll(/\/\*\*\s*(.+?)\s*\*\/\s*\nexport \{ (Icon\w+) \}/g)].map((m) => [m[2], m[1]]);
}

type IconComponent = ComponentType<{ size?: number; stroke?: number }>;

/** Every icon the window uses, at the three sizes, with where it's used */
export const InUse: StoryObj = {
  render: () => (
    <div style={page}>
      <Heading note="From src/components/ui/icons.ts, the one place the window imports icons. Tabler, stroke 1.75.">
        Icons
      </Heading>
      <table style={{ borderCollapse: "collapse", width: "100%" }}>
        <thead>
          <tr>
            <th style={headCell}>Icon</th>
            <th style={headCell}>14px</th>
            <th style={headCell}>16px</th>
            <th style={headCell}>20px</th>
            <th style={headCell}>Used for</th>
          </tr>
        </thead>
        <tbody>
          {uses().map(([name, use]) => {
            const Icon = (icons as unknown as Record<string, IconComponent>)[name];
            return (
              <tr key={name}>
                <td style={cell}>
                  <Name>{name}</Name>
                </td>
                {[14, 16, 20].map((size) => (
                  <td key={size} style={cell}>
                    <Icon size={size} stroke={1.75} />
                  </td>
                ))}
                <td style={{ ...cell, color: "var(--text-secondary)" }}>{use}</td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  ),
};
