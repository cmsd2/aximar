// Keyboard shortcuts in one neutral form, "Mod+Shift+W", read three ways:
// as a Tauri menu accelerator, as a label in the platform's names, and as a
// test against a key press.  "Mod" is Cmd on macOS and Ctrl elsewhere.

export type Platform = "macos" | "windows" | "linux";

/** A shortcut: modifiers then one key, joined by "+", such as "Mod+Shift+Backspace" */
export type Shortcut = string;

const MODIFIERS = ["Ctrl", "Alt", "Shift", "Mod"] as const;
type Modifier = (typeof MODIFIERS)[number];

interface Parsed {
  mods: Set<Modifier>;
  key: string;
}

function parse(s: Shortcut): Parsed {
  const parts = s.split("+");
  // "Mod+Shift++" would mean the + key; Aximar has none, so the last part is the key
  const key = parts.pop() ?? "";
  const mods = new Set<Modifier>();
  for (const p of parts) {
    if (!(MODIFIERS as readonly string[]).includes(p)) throw new Error(`Unknown modifier "${p}" in shortcut "${s}"`);
    mods.add(p as Modifier);
  }
  return { mods, key };
}

let platformOverride: Platform | null = null;

/** The platform the window runs on, from the user agent */
export function currentPlatform(): Platform {
  if (platformOverride) return platformOverride;
  const ua = typeof navigator === "undefined" ? "" : navigator.userAgent;
  if (/Mac/i.test(ua)) return "macos";
  if (/Windows/i.test(ua)) return "windows";
  return "linux";
}

/** For tests and stories: pretend to run on `platform`, or stop with null */
export function setPlatformForTests(platform: Platform | null): void {
  platformOverride = platform;
}

/** The accelerator Tauri's menu builder takes: "CmdOrCtrl+Shift+W" */
export function toAccelerator(s: Shortcut): string {
  const { mods, key } = parse(s);
  const out: string[] = [];
  if (mods.has("Mod")) out.push("CmdOrCtrl");
  if (mods.has("Ctrl")) out.push("Ctrl");
  if (mods.has("Alt")) out.push("Alt");
  if (mods.has("Shift")) out.push("Shift");
  out.push(ACCELERATOR_KEYS[key] ?? key);
  return out.join("+");
}

const ACCELERATOR_KEYS: Record<string, string> = {
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
  ".": "Period",
  "/": "Slash",
  ",": "Comma",
  "[": "BracketLeft",
  "]": "BracketRight",
};

const MAC_KEYS: Record<string, string> = {
  ArrowUp: "↑",
  ArrowDown: "↓",
  ArrowLeft: "←",
  ArrowRight: "→",
  Backspace: "⌫",
  Enter: "↩",
  Escape: "⎋",
  Tab: "⇥",
};

const OTHER_KEYS: Record<string, string> = {
  ArrowUp: "Up",
  ArrowDown: "Down",
  ArrowLeft: "Left",
  ArrowRight: "Right",
};

/**
 * The shortcut in the platform's names: "⇧⌘W" on macOS, with the symbols in
 * Apple's order (⌃⌥⇧⌘), and "Ctrl+Shift+W" on Windows and Linux.
 */
export function formatShortcut(s: Shortcut, platform: Platform = currentPlatform()): string {
  const { mods, key } = parse(s);
  if (platform === "macos") {
    let out = "";
    if (mods.has("Ctrl")) out += "⌃";
    if (mods.has("Alt")) out += "⌥";
    if (mods.has("Shift")) out += "⇧";
    if (mods.has("Mod")) out += "⌘";
    return out + (MAC_KEYS[key] ?? key.toUpperCase());
  }
  const out: string[] = [];
  if (mods.has("Mod") || mods.has("Ctrl")) out.push("Ctrl");
  if (mods.has("Alt")) out.push("Alt");
  if (mods.has("Shift")) out.push("Shift");
  out.push(OTHER_KEYS[key] ?? (key.length === 1 ? key.toUpperCase() : key));
  return out.join("+");
}

/** Whether a key press is this shortcut, on this platform */
export function matches(
  s: Shortcut,
  e: Pick<KeyboardEvent, "key" | "metaKey" | "ctrlKey" | "altKey" | "shiftKey">,
  platform: Platform = currentPlatform(),
): boolean {
  const { mods, key } = parse(s);
  const wantMeta = platform === "macos" && mods.has("Mod");
  const wantCtrl = mods.has("Ctrl") || (platform !== "macos" && mods.has("Mod"));
  if (e.metaKey !== wantMeta || e.ctrlKey !== wantCtrl) return false;
  if (e.altKey !== mods.has("Alt") || e.shiftKey !== mods.has("Shift")) return false;
  return e.key.toLowerCase() === key.toLowerCase();
}
