import type { ComponentType } from "react";
import type { MessageDescriptor } from "@lingui/core";
import { msg } from "@lingui/core/macro";
import { TRANSLATOR_TOOLS } from "../../lib/translation-preview";
import type { SectionProps } from "./SettingRow";
import { GeneralSection } from "./GeneralSection";
import { NotebookSection } from "./NotebookSection";
import { MaximaSection } from "./MaximaSection";
import { PrintingSection } from "./PrintingSection";
import { AiAssistantsSection } from "./AiAssistantsSection";
import { TranslationSection } from "./TranslationSection";

export type SectionId = "general" | "notebook" | "maxima" | "printing" | "ai-assistants" | "translation";

export interface Section {
  id: SectionId;
  label: MessageDescriptor;
  /** Whether this build has the section */
  available: () => boolean;
  Component: ComponentType<SectionProps>;
}

const always = () => true;

/** The Settings window's sections, in sidebar order */
export const SECTIONS: Section[] = [
  {
    id: "general",
    label: msg({ message: "General", comment: "Settings sidebar: language, theme and the variable panel. max-length:16" }),
    available: always,
    Component: GeneralSection,
  },
  {
    id: "notebook",
    label: msg({ message: "Notebook", comment: "Settings sidebar: font size, cell style, argument help and text cells. max-length:16" }),
    available: always,
    Component: NotebookSection,
  },
  {
    id: "maxima",
    label: msg({ message: "Maxima", comment: "Settings sidebar: where Maxima runs and how long a cell may take. max-length:16" }),
    available: always,
    Component: MaximaSection,
  },
  {
    id: "printing",
    label: msg({ message: "Printing", comment: "Settings sidebar: print font size and margins. max-length:16" }),
    available: always,
    Component: PrintingSection,
  },
  {
    id: "ai-assistants",
    label: msg({
      message: "AI assistants",
      comment: "Settings sidebar: the MCP server and the AI tools that use it, such as Claude Code. max-length:16",
    }),
    available: always,
    Component: AiAssistantsSection,
  },
  {
    id: "translation",
    label: msg({
      message: "Translation",
      comment: "Settings sidebar, pre-release builds: tools for translators. max-length:16",
    }),
    available: () => TRANSLATOR_TOOLS,
    Component: TranslationSection,
  },
];

/** The sections this build has */
export function availableSections(): Section[] {
  return SECTIONS.filter((s) => s.available());
}

/** The section to show for a remembered or requested id: General if this build hasn't it */
export function sectionFor(id: string | null | undefined): SectionId {
  return availableSections().find((s) => s.id === id)?.id ?? "general";
}
