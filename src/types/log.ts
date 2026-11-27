export type LogLevel = "info" | "warning" | "error";

export interface LogEntry {
  id: string;
  timestamp: number;
  level: LogLevel;
  message: string;
  source: string;
  /** Somewhere in the window that helps with this entry */
  link?: LogLink;
}

/** A link from a log entry into the window: a Settings section, such as ai-assistants */
export interface LogLink {
  kind: "settings";
  section: string;
}

export interface RawOutputEntry {
  id: string;
  line: string;
  stream: "stdin" | "stdout" | "stderr";
  timestamp: number;
}

/**
 * One frame seen on the kernel-events fd-3 stream — either a parsed
 * envelope (kind set) or a malformed line (parse_error set).
 */
export interface EnvelopeEntry {
  id: string;
  timestampMs: number;
  rawLine: string;
  kind: string | null;
  parseError: string | null;
}

export type LogTab = "app" | "maxima" | "events";
