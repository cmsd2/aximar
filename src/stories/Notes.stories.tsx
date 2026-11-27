import type { Meta, StoryObj } from "@storybook/react-vite";
import { TextWithNotes } from "../components/TextWithNotes";
import { fixtures } from "./fixtures";

const meta: Meta = { title: "Notes" };
export default meta;

/** Every note the core gives, in place of Maxima's lines */
export const AllNotes: StoryObj = {
  render: () => (
    <div>
      {fixtures.notes.map(({ text_output, notes }, i) => (
        <section key={i} style={{ marginBottom: 16 }}>
          <TextWithNotes text={text_output} notes={notes} />
        </section>
      ))}
    </div>
  ),
};
