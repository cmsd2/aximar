// Every icon the window uses, from Tabler (@tabler/icons-react), with
// where it's used.  Import icons from here, never from Tabler itself:
// scripts/check-css-tokens.mjs fails on a direct import, and the
// Foundations icons story lists this file.  Sizes: 14px inline in text,
// 16px in buttons and lists, 20px in the toolbar; stroke 1.75.

/** The type of an icon component, for props that take one */
export type { Icon } from "@tabler/icons-react";

/** Settings sidebar: General */
export { IconAdjustments } from "@tabler/icons-react";
/** Settings sidebar: Notebook */
export { IconNotebook } from "@tabler/icons-react";
/** Settings sidebar: Maxima */
export { IconMathFunction } from "@tabler/icons-react";
/** Settings sidebar: Printing */
export { IconPrinter } from "@tabler/icons-react";
/** Settings sidebar: AI assistants */
export { IconPlugConnected } from "@tabler/icons-react";
/** Settings sidebar: Translation */
export { IconLanguage } from "@tabler/icons-react";
/** Close buttons: the Settings window */
export { IconX } from "@tabler/icons-react";
/** StatusBox: ok */
export { IconCircleCheck } from "@tabler/icons-react";
/** StatusBox: warn and err */
export { IconAlertTriangle } from "@tabler/icons-react";
/** StatusBox: info */
export { IconInfoCircle } from "@tabler/icons-react";
/** StatusBox: busy, spinning */
export { IconLoader2 } from "@tabler/icons-react";
